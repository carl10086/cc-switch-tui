use cc_switch_tui::templates::register_templates;

#[test]
fn test_minimax_template_registered() {
    let templates = register_templates();
    assert_eq!(templates.len(), 2);

    let minimax = templates.iter().find(|t| t.id == "minimax").unwrap();
    assert_eq!(minimax.name, "MiniMax");
    assert_eq!(
        minimax.default_env.get("ANTHROPIC_BASE_URL").unwrap(),
        "https://api.minimaxi.com/anthropic"
    );
    assert_eq!(minimax.models.len(), 2);

    let model = &minimax.models[0];
    assert_eq!(model.id, "MiniMax-M3");
    assert_eq!(
        model.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "MiniMax-M3"
    );
    assert_eq!(
        model.env_overrides.get("ANTHROPIC_DEFAULT_OPUS_MODEL").unwrap(),
        "MiniMax-M3"
    );

    let model2 = &minimax.models[1];
    assert_eq!(model2.id, "MiniMax-M2.7-highspeed");
    assert_eq!(
        model2.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "MiniMax-M2.7-highspeed"
    );
    assert_eq!(
        model2
            .env_overrides
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .unwrap(),
        "MiniMax-M2.7-highspeed"
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
    assert_eq!(kimi.models.len(), 1);
    assert_eq!(kimi.opencode_models, vec!["kimi-for-coding".to_string()]);

    let model = &kimi.models[0];
    assert_eq!(model.id, "kimi-for-coding");
    assert_eq!(model.name, "Kimi for Coding");
    assert_eq!(model.opencode_model_id, "kimi-for-coding");
    assert_eq!(
        model.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "kimi-for-coding"
    );
    assert_eq!(
        model
            .env_overrides
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .unwrap(),
        "kimi-for-coding"
    );
    assert_eq!(
        model
            .env_overrides
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .unwrap(),
        "kimi-for-coding"
    );
    assert_eq!(
        model
            .env_overrides
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .unwrap(),
        "kimi-for-coding"
    );
}
