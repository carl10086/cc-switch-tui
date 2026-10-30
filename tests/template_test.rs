use cc_switch_tui::templates::register_templates;

#[test]
fn test_minimax_template_registered() {
    let templates = register_templates();
    assert_eq!(templates.len(), 2);

    let minimax = templates.iter().find(|t| t.id == "minimax").unwrap();
    assert_eq!(minimax.name, "MiniMax");
    assert_eq!(
        minimax.default_env.get("ANTHROPIC_BASE_URL").unwrap(),
        "https://api.minimax.cn/anthropic"
    );
    // MiniMax 只保留唯一档位：最新旗舰 MiniMax-M3.1-Flash[1m]
    assert_eq!(minimax.models.len(), 1);

    let model = &minimax.models[0];
    assert_eq!(model.id, "MiniMax-M3.1-Flash[1m]");
    assert_eq!(model.name, "MiniMax M3.1 Flash [1m]");
    assert_eq!(
        model.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "MiniMax-M3.1-Flash[1m]"
    );
    assert_eq!(
        model
            .env_overrides
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .unwrap(),
        "MiniMax-M3.1-Flash[1m]"
    );
    assert_eq!(
        model
            .env_overrides
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .unwrap(),
        "MiniMax-M3.1-Flash[1m]"
    );
    assert_eq!(
        model
            .env_overrides
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .unwrap(),
        "MiniMax-M3.1-Flash[1m]"
    );
    // 官方推荐的 512K 自动压缩阈值 + 模型真实 1M 窗口
    assert_eq!(
        model
            .env_overrides
            .get("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
            .unwrap(),
        "524288"
    );
    assert_eq!(
        model
            .env_overrides
            .get("CLAUDE_CODE_MAX_CONTEXT_TOKENS")
            .unwrap(),
        "1048576"
    );
    // M3.1-Flash 支持 thinking depth 调档
    assert_eq!(
        model.env_overrides.get("CLAUDE_CODE_EFFORT_LEVEL").unwrap(),
        "max"
    );
}

#[test]
fn test_kimi_template_registered() {
    let templates = register_templates();
    assert_eq!(templates.len(), 2);

    let kimi = templates.iter().find(|t| t.id == "kimi").unwrap();
    assert_eq!(kimi.name, "Kimi");
    assert_eq!(
        kimi.default_env.get("ANTHROPIC_BASE_URL").unwrap(),
        "https://api.kimi.com/coding/"
    );
    assert_eq!(kimi.models.len(), 3);

    // k3[1m] 排在第一位，作为默认 model
    let k3 = &kimi.models[0];
    assert_eq!(k3.id, "k3[1m]");
    assert_eq!(k3.name, "Kimi K3 [1m]");
    assert_eq!(k3.env_overrides.get("ANTHROPIC_MODEL").unwrap(), "k3[1m]");
    assert_eq!(
        k3.env_overrides.get("CLAUDE_CODE_EFFORT_LEVEL").unwrap(),
        "high"
    );
    assert_eq!(
        k3.env_overrides
            .get("CLAUDE_CODE_MAX_CONTEXT_TOKENS")
            .unwrap(),
        "1048576"
    );

    let highspeed = &kimi.models[1];
    assert_eq!(highspeed.id, "kimi-for-coding-highspeed");
    assert_eq!(highspeed.name, "Kimi for Coding · Highspeed");
    assert_eq!(
        highspeed.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "kimi-for-coding-highspeed"
    );
    assert_eq!(
        highspeed
            .env_overrides
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .unwrap(),
        "kimi-for-coding-highspeed"
    );
    assert_eq!(
        highspeed
            .env_overrides
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .unwrap(),
        "kimi-for-coding-highspeed"
    );
    assert_eq!(
        highspeed
            .env_overrides
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .unwrap(),
        "kimi-for-coding-highspeed"
    );

    let normal = &kimi.models[2];
    assert_eq!(normal.id, "kimi-for-coding");
    assert_eq!(normal.name, "Kimi for Coding");
    assert_eq!(
        normal.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "kimi-for-coding"
    );
    assert_eq!(
        normal
            .env_overrides
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .unwrap(),
        "kimi-for-coding"
    );
    assert_eq!(
        normal
            .env_overrides
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .unwrap(),
        "kimi-for-coding"
    );
    assert_eq!(
        normal
            .env_overrides
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .unwrap(),
        "kimi-for-coding"
    );
}
