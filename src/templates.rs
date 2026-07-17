use crate::domain::{ModelTemplate, ProviderTemplate};
use std::collections::HashMap;

/// 构建 6 个模型路由槽位 env vars（MODEL + 4 个 DEFAULT_* + SUBAGENT），
/// 全部指向同一 `model_id`。避免 Claude Code 用默认名发请求被后端拒识。
///
/// per-provider 上下文参数（`AUTO_COMPACT_WINDOW` / `MAX_CONTEXT_TOKENS`）和
/// `EFFORT_LEVEL`（仅 k3）由 caller 在返回 HashMap 上追加，保持各自独立。
fn routing_env_vars(model_id: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();
    let slots = [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_FABLE_MODEL",
    ];
    for key in slots {
        env.insert(key.to_string(), model_id.to_string());
    }
    env.insert(
        "CLAUDE_CODE_SUBAGENT_MODEL".to_string(),
        model_id.to_string(),
    );
    env
}

/// 构建 1M/256K 对齐的上下文参数 env vars（`AUTO_COMPACT_WINDOW` +
/// `MAX_CONTEXT_TOKENS`），均取同一 `window` 值（与 model 上下文大小一致）。
fn context_env_vars(window: u32) -> HashMap<String, String> {
    let w = window.to_string();
    [
        ("CLAUDE_CODE_AUTO_COMPACT_WINDOW", w.clone()),
        ("CLAUDE_CODE_MAX_CONTEXT_TOKENS", w),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

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
    default_env.insert("API_TIMEOUT_MS".to_string(), "3000000".to_string());
    default_env.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        "1".to_string(),
    );

    // 跟随 MiniMax 官方 2026 Claude Code 集成文档：
    //   - model id 使用 MiniMax-M3[1m]（含 [1m] 后缀表示 1M 上下文）
    //   - CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000 / MAX_CONTEXT_TOKENS=1000000
    //     与 1M 窗口对齐
    // 配置完全由 model id 决定（env_overrides 字面量），不再走 instance toggle。
    let mut env_overrides_m3 = routing_env_vars("MiniMax-M3[1m]");
    env_overrides_m3.extend(context_env_vars(1000000));

    let mut env_overrides_m27 = routing_env_vars("MiniMax-M2.7-highspeed");
    env_overrides_m27.extend(context_env_vars(1000000));

    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        default_env,
        models: vec![
            ModelTemplate {
                id: "MiniMax-M3[1m]".to_string(),
                name: "MiniMax M3 [1m]".to_string(),
                env_overrides: env_overrides_m3,
                opencode_model_id: "MiniMax-M3[1m]".to_string(),
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
        opencode_models: vec![
            "MiniMax-M2.7-highspeed".to_string(),
            "MiniMax-M3[1m]".to_string(),
        ],
    }
}

/// 构建 kimi Provider 模板
fn kimi_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.kimi.com/coding/".to_string(),
    );

    // Kimi 官方建议第三方工具统一使用 stable alias 作为请求体 model 字段；
    // 后端会自动映射到最新发布的模型。与 MiniMax 模式对齐，显式注入模型路由
    // 槽位 env，避免 Claude Code 用默认 model 名发请求被 Kimi 后端拒识。
    //
    // 档位（2026-07 跟随 Kimi 官方档位表）：
    //   - k3[1m]                   2026-07-16 发布：显式 model id（非 alias），1M context
    //   - kimi-for-coding-highspeed 高速版：5–6× 输出速度、3× 额度、Allegretto+ 会员
    //   - kimi-for-coding          普通版：所有会员可用，基准速度
    //
    // k3[1m] env 跟随官方建议：6 个模型路由槽位 + CLAUDE_CODE_EFFORT_LEVEL=max
    // （思考程度，当前仅 k3 支持）+ 1M 窗口对齐（1048576）。
    let mut env_overrides_k3 = routing_env_vars("k3[1m]");
    env_overrides_k3.insert("CLAUDE_CODE_EFFORT_LEVEL".to_string(), "max".to_string());
    env_overrides_k3.extend(context_env_vars(1048576));

    let mut env_overrides_highspeed = routing_env_vars("kimi-for-coding-highspeed");
    env_overrides_highspeed.extend(context_env_vars(262144));

    let mut env_overrides_normal = routing_env_vars("kimi-for-coding");
    env_overrides_normal.extend(context_env_vars(262144));

    ProviderTemplate {
        id: "kimi".to_string(),
        name: "Kimi".to_string(),
        default_env,
        models: vec![
            // k3[1m] 排在第一位，作为默认 model
            ModelTemplate {
                id: "k3[1m]".to_string(),
                name: "Kimi K3 [1m]".to_string(),
                env_overrides: env_overrides_k3,
                opencode_model_id: "k3[1m]".to_string(),
            },
            ModelTemplate {
                id: "kimi-for-coding-highspeed".to_string(),
                name: "Kimi for Coding · Highspeed".to_string(),
                env_overrides: env_overrides_highspeed,
                opencode_model_id: "kimi-for-coding-highspeed".to_string(),
            },
            ModelTemplate {
                id: "kimi-for-coding".to_string(),
                name: "Kimi for Coding".to_string(),
                env_overrides: env_overrides_normal,
                opencode_model_id: "kimi-for-coding".to_string(),
            },
        ],
        opencode_provider_id: "kimi-for-coding".to_string(),
        opencode_npm: "@ai-sdk/anthropic".to_string(),
        opencode_base_url: "https://api.kimi.com/coding/v1".to_string(),
        opencode_env_var: "KIMI_API_KEY".to_string(),
        opencode_models: vec![
            "k3[1m]".to_string(),
            "kimi-for-coding-highspeed".to_string(),
            "kimi-for-coding".to_string(),
        ],
    }
}
