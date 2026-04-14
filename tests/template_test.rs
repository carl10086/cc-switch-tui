use cc_switch_tui::app::templates::register_templates;

#[test]
fn test_minimax_template_registered() {
    let templates = register_templates();
    assert_eq!(templates.len(), 1);

    let minimax = templates.iter().find(|t| t.id == "minimax").unwrap();
    assert_eq!(minimax.name, "MiniMax");
    assert_eq!(
        minimax.default_env.get("ANTHROPIC_BASE_URL").unwrap(),
        "https://api.minimaxi.com/anthropic"
    );
    assert_eq!(minimax.models.len(), 1);

    let model = &minimax.models[0];
    assert_eq!(model.id, "MiniMax-M2.7-highspeed");
    assert_eq!(
        model.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
        "MiniMax-M2.7-highspeed"
    );
}
