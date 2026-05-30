use serde::Deserialize;
use std::collections::HashMap;

/// OpenCode 模型 API 端点
const MODELS_API_URL: &str = "https://models.dev/api.json";

/// 缓存的模型信息，key 为 provider_id，value 为该 provider 支持的 model ID 列表
pub type OpencodeModelCache = HashMap<String, Vec<String>>;

/// 从 models.dev/api.json 拉取所有 provider 的模型列表
///
/// 失败时返回 Err(错误描述)，由调用方决定是否展示给用户
pub fn fetch_opencode_models() -> Result<OpencodeModelCache, String> {
    let response = ureq::get(MODELS_API_URL)
        .call()
        .map_err(|e| format!("fetch opencode models failed: {}", e))?;

    let body = response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("read opencode models response failed: {}", e))?;

    let parsed: HashMap<String, ProviderResponse> = serde_json::from_str(&body)
        .map_err(|e| format!("parse opencode models json failed: {}", e))?;

    let mut cache = HashMap::new();
    for (provider_id, provider) in parsed {
        if !is_valid_id(&provider_id) {
            tracing::warn!("skipping provider with invalid id: {}", provider_id);
            continue;
        }
        let models: Vec<String> = provider
            .models
            .keys()
            .filter(|id| is_valid_id(id))
            .cloned()
            .collect();
        if !models.is_empty() {
            cache.insert(provider_id, models);
        }
    }

    tracing::info!("opencode models cached: {} providers", cache.len());
    Ok(cache)
}

/// 验证 provider/model ID 只包含安全字符（字母数字、连字符、下划线）
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[derive(Debug, Deserialize)]
struct ProviderResponse {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    npm: Option<String>,
    #[allow(dead_code)]
    api: Option<String>,
    #[allow(dead_code)]
    env: Option<Vec<String>>,
    models: HashMap<String, ModelResponse>,
}

#[derive(Debug, Deserialize)]
struct ModelResponse {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "需要网络访问 models.dev"]
    fn test_fetch_opencode_models_live() {
        let cache = fetch_opencode_models().expect("fetch should succeed in live test");
        println!("Cached providers: {:?}", cache.keys().collect::<Vec<_>>());
        if let Some(models) = cache.get("minimax-cn") {
            println!("minimax-cn models: {:?}", models);
        }
        if let Some(models) = cache.get("kimi-for-coding") {
            println!("kimi-for-coding models: {:?}", models);
        }
        assert!(!cache.is_empty(), "Cache should not be empty");
    }
}
