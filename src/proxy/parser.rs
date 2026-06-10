//! Anthropic API payload parser.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::proxy::sse::SseEvent;

/// A parsed message from the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Parsed Anthropic request payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParsedRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub system: Option<String>,
}

/// Parsed Anthropic response payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParsedResponse {
    pub content: String,
    pub stop_reason: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub model: String,
}

/// Accumulates streaming SSE events into a partial response.
#[derive(Debug, Clone, Default)]
pub struct StreamingAccumulator {
    pub content: Vec<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub stop_reason: Option<String>,
    pub model: String,
}

/// Parser for Anthropic request/response payloads.
pub struct AnthropicParser;

impl AnthropicParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a non-streaming request JSON body.
    pub fn parse_request(&self, payload: &str) -> ParsedRequest {
        let mut result = ParsedRequest::default();
        let Ok(json) = serde_json::from_str::<Value>(payload) else {
            return result;
        };

        result.model = json
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        result.max_tokens = json.get("max_tokens").and_then(Value::as_u64).map(|v| v as u32);
        result.system = json
            .get("system")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        if let Some(arr) = json.get("messages").and_then(Value::as_array) {
            result.messages = arr
                .iter()
                .filter_map(|msg| {
                    let role = msg.get("role")?.as_str()?.to_string();
                    let content = msg.get("content")?.as_str()?.to_string();
                    Some(Message { role, content })
                })
                .collect();
        }

        result
    }

    /// Parse a non-streaming response JSON body.
    pub fn parse_response(&self, payload: &str) -> ParsedResponse {
        let mut result = ParsedResponse::default();
        let Ok(json) = serde_json::from_str::<Value>(payload) else {
            return result;
        };

        if let Some(arr) = json.get("content").and_then(Value::as_array) {
            let texts: Vec<String> = arr
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str).map(|s| s.to_string()))
                .collect();
            result.content = texts.join("");
        }

        result.stop_reason = json
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        result.model = json
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        if let Some(usage) = json.get("usage") {
            result.input_tokens = usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32);
            result.output_tokens = usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .map(|v| v as u32);
        }

        result
    }

    /// Parse a single SSE event and update the accumulator.
    pub fn apply_streaming_event(&self,
        acc: &mut StreamingAccumulator,
        event: &SseEvent,
    ) {
        match event.event_type.as_deref() {
            Some("message_start") => {
                if let Ok(json) = serde_json::from_str::<Value>(&event.data) {
                    if let Some(model) = json
                        .get("message")
                        .and_then(|m| m.get("model"))
                        .and_then(Value::as_str)
                    {
                        acc.model = model.to_string();
                    }
                }
            }
            Some("content_block_delta") => {
                if let Ok(json) = serde_json::from_str::<Value>(&event.data) {
                    if let Some(text) = json
                        .get("delta")
                        .and_then(|d| d.get("text"))
                        .and_then(Value::as_str)
                    {
                        acc.content.push(text.to_string());
                    }
                }
            }
            Some("message_delta") => {
                if let Ok(json) = serde_json::from_str::<Value>(&event.data) {
                    if let Some(usage) = json.get("usage") {
                        acc.input_tokens = usage
                            .get("input_tokens")
                            .and_then(Value::as_u64)
                            .map(|v| v as u32);
                        acc.output_tokens = usage
                            .get("output_tokens")
                            .and_then(Value::as_u64)
                            .map(|v| v as u32);
                    }
                    if let Some(reason) = json
                        .get("delta")
                        .and_then(|d| d.get("stop_reason"))
                        .and_then(Value::as_str)
                    {
                        acc.stop_reason = Some(reason.to_string());
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let parser = AnthropicParser::new();
        let payload = r#"{
            "model": "claude-sonnet-4-6",
            "max_tokens": 4096,
            "system": "You are a helpful assistant.",
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there!"}
            ]
        }"#;

        let req = parser.parse_request(payload);
        assert_eq!(req.model, "claude-sonnet-4-6");
        assert_eq!(req.max_tokens, Some(4096));
        assert_eq!(req.system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[0].content, "Hello");
    }

    #[test]
    fn test_parse_request_invalid_json() {
        let parser = AnthropicParser::new();
        let req = parser.parse_request("not json");
        assert_eq!(req, ParsedRequest::default());
    }

    #[test]
    fn test_parse_response() {
        let parser = AnthropicParser::new();
        let payload = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-6",
            "content": [
                {"type": "text", "text": "Hello world"}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 20}
        }"#;

        let resp = parser.parse_response(payload);
        assert_eq!(resp.content, "Hello world");
        assert_eq!(resp.model, "claude-sonnet-4-6");
        assert_eq!(resp.stop_reason, Some("end_turn".to_string()));
        assert_eq!(resp.input_tokens, Some(10));
        assert_eq!(resp.output_tokens, Some(20));
    }

    #[test]
    fn test_parse_response_invalid_json() {
        let parser = AnthropicParser::new();
        let resp = parser.parse_response("not json");
        assert_eq!(resp, ParsedResponse::default());
    }

    #[test]
    fn test_streaming_message_start() {
        let parser = AnthropicParser::new();
        let mut acc = StreamingAccumulator::default();
        let event = SseEvent {
            event_type: Some("message_start".to_string()),
            data: r#"{"message":{"id":"msg_123","type":"message","role":"assistant","model":"claude-sonnet-4-6"}}"#.to_string(),
        };
        parser.apply_streaming_event(&mut acc, &event);
        assert_eq!(acc.model, "claude-sonnet-4-6");
    }

    #[test]
    fn test_streaming_content_block_delta() {
        let parser = AnthropicParser::new();
        let mut acc = StreamingAccumulator::default();
        let event = SseEvent {
            event_type: Some("content_block_delta".to_string()),
            data: r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#.to_string(),
        };
        parser.apply_streaming_event(&mut acc, &event);
        assert_eq!(acc.content, vec!["Hello"]);
    }

    #[test]
    fn test_streaming_message_delta() {
        let parser = AnthropicParser::new();
        let mut acc = StreamingAccumulator::default();
        let event = SseEvent {
            event_type: Some("message_delta".to_string()),
            data: r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":10,"output_tokens":20}}"#.to_string(),
        };
        parser.apply_streaming_event(&mut acc, &event);
        assert_eq!(acc.stop_reason, Some("end_turn".to_string()));
        assert_eq!(acc.input_tokens, Some(10));
        assert_eq!(acc.output_tokens, Some(20));
    }
}
