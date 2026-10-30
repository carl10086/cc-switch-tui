use crate::domain::{ModelTemplate, ProviderTemplate};
use std::collections::HashMap;

/// 构建 6 个模型路由槽位 env vars（MODEL + 4 个 DEFAULT_* + SUBAGENT），
/// 全部指向同一 `model_id`。避免 Claude Code 用默认名发请求被后端拒识。
///
/// per-provider 上下文参数（`AUTO_COMPACT_WINDOW` / `MAX_CONTEXT_TOKENS`）和
/// `EFFORT_LEVEL`（仅部分 provider 支持）由 caller 在返回 HashMap 上追加，保持各自独立。
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

/// 构建上下文参数 env vars，两个值语义不同、不可绑成同一个：
/// - `auto_compact_window`：自动压缩触发阈值（Claude Code 到该 token 数时开始压缩）
/// - `max_context_tokens`：模型上下文窗口上限（须与模型真实能力一致）
///
/// 例：MiniMax M3.1-Flash 真实窗口 1M，但官方推荐在 512K 处自动压缩。
fn context_env_vars(auto_compact_window: u32, max_context_tokens: u32) -> HashMap<String, String> {
    [
        (
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
            auto_compact_window.to_string(),
        ),
        (
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
            max_context_tokens.to_string(),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// MiniMax M3.1-Flash 的上下文参数：
/// - 真实窗口 1M（models.dev 与 MiniMax 文档均为 1,000,000）
/// - 自动压缩阈值 512K（MiniMax 官方 Claude Code 文档的推荐值）
const MINIMAX_AUTO_COMPACT_WINDOW: u32 = 524288;
const MINIMAX_MAX_CONTEXT_TOKENS: u32 = 1048576;

/// 注册并返回所有内置的 Provider 模板
pub fn register_templates() -> Vec<ProviderTemplate> {
    vec![minimax_template(), kimi_template()]
}

/// 构建 minimax Provider 模板
fn minimax_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.minimax.cn/anthropic".to_string(),
    );
    default_env.insert("API_TIMEOUT_MS".to_string(), "3000000".to_string());
    default_env.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        "1".to_string(),
    );

    // 跟随 MiniMax 官方 Claude Code 集成文档（国内站 api.minimax.cn）：
    //   - model id 使用 MiniMax-M3.1-Flash[1m]（含 [1m] 后缀表示 1M 上下文）
    //   - CLAUDE_CODE_AUTO_COMPACT_WINDOW=524288 —— 官方推荐的 512K 自动压缩阈值
    //   - CLAUDE_CODE_MAX_CONTEXT_TOKENS=1048576 —— 模型真实窗口 1M
    //   - CLAUDE_CODE_EFFORT_LEVEL=max —— M3.1-Flash 支持 thinking depth 调档
    //     （low/medium/high/xhigh/max，省略时后端默认就是 max）
    // 配置完全由 model id 决定（env_overrides 字面量），不再走 instance toggle。
    let mut env_overrides_m31 = routing_env_vars("MiniMax-M3.1-Flash[1m]");
    env_overrides_m31.insert("CLAUDE_CODE_EFFORT_LEVEL".to_string(), "max".to_string());
    env_overrides_m31.extend(context_env_vars(
        MINIMAX_AUTO_COMPACT_WINDOW,
        MINIMAX_MAX_CONTEXT_TOKENS,
    ));

    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        default_env,
        models: vec![ModelTemplate {
            id: "MiniMax-M3.1-Flash[1m]".to_string(),
            name: "MiniMax M3.1 Flash [1m]".to_string(),
            env_overrides: env_overrides_m31,
        }],
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
    // k3[1m] env 跟随官方建议：6 个模型路由槽位 + CLAUDE_CODE_EFFORT_LEVEL=high
    // （思考程度，当前仅 k3 支持）+ 1M 窗口对齐（1048576）。
    let mut env_overrides_k3 = routing_env_vars("k3[1m]");
    env_overrides_k3.insert("CLAUDE_CODE_EFFORT_LEVEL".to_string(), "high".to_string());
    env_overrides_k3.extend(context_env_vars(1048576, 1048576));

    let mut env_overrides_highspeed = routing_env_vars("kimi-for-coding-highspeed");
    env_overrides_highspeed.extend(context_env_vars(262144, 262144));

    let mut env_overrides_normal = routing_env_vars("kimi-for-coding");
    env_overrides_normal.extend(context_env_vars(262144, 262144));

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
            },
            ModelTemplate {
                id: "kimi-for-coding-highspeed".to_string(),
                name: "Kimi for Coding · Highspeed".to_string(),
                env_overrides: env_overrides_highspeed,
            },
            ModelTemplate {
                id: "kimi-for-coding".to_string(),
                name: "Kimi for Coding".to_string(),
                env_overrides: env_overrides_normal,
            },
        ],
    }
}
