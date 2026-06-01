use crate::domain::{ModelTemplate, ProviderTemplate};
use std::collections::HashMap;

/// 注册并返回所有内置的 Provider 模板
pub fn register_templates() -> Vec<ProviderTemplate> {
    vec![minimax_template(), kimi_template()]
}

/// 构建 minimax Provider 模板
fn minimax_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.minimaxi.com/anthropic".to_string(),
    );
    default_env.insert(
        "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
        "MiniMax-M3".to_string(),
    );
    default_env.insert(
        "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
        "MiniMax-M3".to_string(),
    );
    default_env.insert(
        "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
        "MiniMax-M3".to_string(),
    );
    default_env.insert("API_TIMEOUT_MS".to_string(), "3000000".to_string());
    default_env.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        "1".to_string(),
    );

    let mut env_overrides_m3 = HashMap::new();
    env_overrides_m3.insert(
        "ANTHROPIC_MODEL".to_string(),
        "MiniMax-M3".to_string(),
    );

    let mut env_overrides_m27 = HashMap::new();
    env_overrides_m27.insert(
        "ANTHROPIC_MODEL".to_string(),
        "MiniMax-M2.7-highspeed".to_string(),
    );

    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        default_env,
        models: vec![
            ModelTemplate {
                id: "MiniMax-M3".to_string(),
                name: "MiniMax M3".to_string(),
                env_overrides: env_overrides_m3,
                opencode_model_id: "MiniMax-M3".to_string(),
            },
            ModelTemplate {
                id: "MiniMax-M2.7-highspeed".to_string(),
                name: "MiniMax M2.7 Highspeed".to_string(),
                env_overrides: env_overrides_m27,
                opencode_model_id: "MiniMax-M2.7-highspeed".to_string(),
            },
        ],
        opencode_provider_id: "minimax-cn".to_string(),
        opencode_npm: "@ai-sdk/anthropic".to_string(),
        opencode_base_url: "https://api.minimaxi.com/anthropic/v1".to_string(),
        opencode_env_var: "MINIMAX_API_KEY".to_string(),
    }
}

/// 构建 kimi Provider 模板
fn kimi_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.kimi.com/coding/".to_string(),
    );

    ProviderTemplate {
        id: "kimi".to_string(),
        name: "Kimi".to_string(),
        default_env,
        models: vec![ModelTemplate {
            id: "kimi-for-coding".to_string(),
            name: "Kimi for Coding".to_string(),
            env_overrides: HashMap::new(),
            opencode_model_id: "k2p5".to_string(),
        }],
        opencode_provider_id: "kimi-for-coding".to_string(),
        opencode_npm: "@ai-sdk/anthropic".to_string(),
        opencode_base_url: "https://api.kimi.com/coding/v1".to_string(),
        opencode_env_var: "KIMI_API_KEY".to_string(),
    }
}
