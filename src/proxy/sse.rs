//! SSE (Server-Sent Events) parser.

/// A single parsed SSE event.
#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
}

/// Parser for SSE byte streams.
///
/// SSE events are separated by empty lines. Each event consists of
/// field lines (`event:`, `data:`, etc.). The parser buffers partial
/// input across chunks and emits complete events.
pub struct SseParser {
    buffer: String,
    current_lines: Vec<String>,
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            current_lines: Vec::new(),
        }
    }

    /// Feed a chunk of bytes and return any complete events.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));

        let mut events = Vec::new();

        while let Some(pos) = self.buffer.find('\n') {
            let mut line = self.buffer[..pos].to_string();
            self.buffer.drain(..pos + 1);

            // Strip trailing \r for \r\n line endings
            if line.ends_with('\r') {
                line.pop();
            }

            if line.is_empty() {
                if let Some(event) = Self::parse_lines(&self.current_lines) {
                    events.push(event);
                }
                self.current_lines.clear();
            } else {
                self.current_lines.push(line);
            }
        }

        events
    }

    /// Flush any remaining buffered data as final events.
    pub fn flush(&mut self) -> Vec<SseEvent> {
        // An empty line terminates the current event; inject two to ensure closure.
        self.feed(b"\n\n")
    }

    fn parse_lines(lines: &[String]) -> Option<SseEvent> {
        let mut event_type = None;
        let mut data_parts: Vec<&str> = Vec::new();

        for line in lines {
            if let Some(t) = line.strip_prefix("event:") {
                event_type = Some(t.trim().to_string());
            } else if let Some(d) = line.strip_prefix("data:") {
                data_parts.push(d.trim_start());
            }
            // Ignore `id:`, `retry:`, comments (`:`), and unknown fields
        }

        if data_parts.is_empty() {
            return None;
        }

        Some(SseEvent {
            event_type,
            data: data_parts.join("\n"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_event() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"data: {\"type\":\"test\"}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, None);
        assert_eq!(events[0].data, "{\"type\":\"test\"}");
    }

    #[test]
    fn test_event_with_type() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"event: message_start\ndata: {\"type\":\"start\"}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, Some("message_start".to_string()));
        assert_eq!(events[0].data, "{\"type\":\"start\"}");
    }

    #[test]
    fn test_multiple_events() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"data: first\n\ndata: second\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_split_across_chunks() {
        let mut parser = SseParser::new();
        let events1 = parser.feed(b"data: hel");
        assert!(events1.is_empty());

        let events2 = parser.feed(b"lo\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "hello");
    }

    #[test]
    fn test_crlf_line_endings() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"data: test\r\n\r\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "test");
    }

    #[test]
    fn test_multiple_data_lines() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"data: hello\ndata: world\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello\nworld");
    }

    #[test]
    fn test_flush_incomplete() {
        let mut parser = SseParser::new();
        let events = parser.feed(b"data: incomplete");
        assert!(events.is_empty());

        let flushed = parser.flush();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].data, "incomplete");
    }

    #[test]
    fn test_ignores_comments_and_unknown_fields() {
        let mut parser = SseParser::new();
        let events = parser.feed(b":comment\ndata: value\nid: 123\nretry: 5000\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "value");
        assert_eq!(events[0].event_type, None);
    }
}
