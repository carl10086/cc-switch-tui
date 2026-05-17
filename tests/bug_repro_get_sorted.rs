use cc_switch_tui::app::state::App;
use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;
use cc_switch_tui::dao::Dao;
use cc_switch_tui::domain::{ModelTemplate, ProviderInstance, ProviderTemplate};
use std::collections::HashMap;

fn test_templates() -> Vec<ProviderTemplate> {
    vec![
        ProviderTemplate {
            id: "minimax".to_string(),
            name: "MiniMax".to_string(),
            default_env: HashMap::new(),
            models: vec![ModelTemplate {
                id: "m1".to_string(),
                name: "Model 1".to_string(),
                env_overrides: HashMap::new(),
                opencode_model_id: String::new(),
            }],
            opencode_provider_id: String::new(),
            opencode_npm: String::new(),
            opencode_base_url: String::new(),
            opencode_env_var: String::new(),
        },
        ProviderTemplate {
            id: "kimi".to_string(),
            name: "Kimi".to_string(),
            default_env: HashMap::new(),
            models: vec![ModelTemplate {
                id: "kimi-for-coding".to_string(),
                name: "Kimi for Coding".to_string(),
                env_overrides: HashMap::new(),
                opencode_model_id: String::new(),
            }],
            opencode_provider_id: String::new(),
            opencode_npm: String::new(),
            opencode_base_url: String::new(),
            opencode_env_var: String::new(),
        },
    ]
}

#[test]
fn test_get_sorted_instances_returns_all_instances_with_alias_ids() {
    let templates = test_templates();
    let dao = MemoryDaoImpl::new(templates);
    let mut app = App::new_with_dao(dao);

    // Create 2 kimi instances with alias-based IDs
    let i1 = ProviderInstance {
        id: "kimi-kimi-for-coding-cl-km2".to_string(),
        template_id: "kimi".to_string(),
        model_id: "kimi-for-coding".to_string(),
        api_key: "key1".to_string(),
        created_at: chrono::Utc::now() - chrono::Duration::seconds(10),
        alias: "cl-km2".to_string(),
        opencode_model_id: String::new(),
    };
    let i2 = ProviderInstance {
        id: "kimi-kimi-for-coding-cl-km3".to_string(),
        template_id: "kimi".to_string(),
        model_id: "kimi-for-coding".to_string(),
        api_key: "key2".to_string(),
        created_at: chrono::Utc::now(),
        alias: "cl-km3".to_string(),
        opencode_model_id: String::new(),
    };

    app.dao.create_instance(i1).unwrap();
    app.dao.create_instance(i2).unwrap();

    let sorted = app.get_sorted_instances();
    assert_eq!(
        sorted.len(),
        2,
        "get_sorted_instances should return all 2 instances, but got {}",
        sorted.len()
    );
}
