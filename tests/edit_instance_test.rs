use cc_switch_tui::app::state::{App, AppState};
use cc_switch_tui::dao::Dao;
use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;
use cc_switch_tui::domain::{ModelTemplate, ProviderInstance, ProviderTemplate};
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;

fn minimax_template() -> ProviderTemplate {
    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        default_env: HashMap::new(),
        models: vec![
            ModelTemplate {
                id: "MiniMax-M2.7-highspeed".to_string(),
                name: "MiniMax M2.7 Highspeed".to_string(),
                env_overrides: HashMap::new(),
                opencode_model_id: "MiniMax-M2.7-highspeed".to_string(),
            },
            ModelTemplate {
                id: "MiniMax-M3".to_string(),
                name: "MiniMax M3".to_string(),
                env_overrides: HashMap::new(),
                opencode_model_id: "MiniMax-M3".to_string(),
            },
        ],
        opencode_provider_id: "minimax-cn".to_string(),
        opencode_npm: "@ai-sdk/anthropic".to_string(),
        opencode_base_url: "https://api.minimaxi.com/anthropic/v1".to_string(),
        opencode_env_var: "MINIMAX_API_KEY".to_string(),
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

/// 端到端：通过 EditInfoPanel 改 model 后，model_id 更新，id 保持稳定。
#[test]
fn test_edit_model_via_info_panel() {
    let templates = vec![minimax_template()];
    let dao = MemoryDaoImpl::new(templates);
    let mut app = App::new_with_dao(dao);

    // 预置一个 instance
    let inst = ProviderInstance {
        id: "minimax-cl-mini".to_string(),
        template_id: "minimax".to_string(),
        model_id: "MiniMax-M2.7-highspeed".to_string(),
        api_key: "sk-test".to_string(),
        created_at: Utc::now(),
        alias: "cl-mini".to_string(),
        opencode_model_id: String::new(),
        kv_cache_enabled: false,
    };
    app.dao.create_instance(inst).unwrap();

    // 模拟用户操作：进 EditInfoPanel → Down×1 到 Model 行 → Enter 进 EditModel → Down×1 选 M3 → Enter 保存
    app.state = AppState::EditInfoPanel {
        instance_id: "minimax-cl-mini".to_string(),
        focus_index: 0, // Model
    };
    app.on_key(key(KeyCode::Enter)); // 进 EditModel
    assert!(matches!(app.state, AppState::EditModel { .. }));

    // 选 M2.7 highspeed 时 model_index=0，按 Down 切到 M3
    assert_eq!(app.model_index, 0);
    app.on_key(key(KeyCode::Down));
    assert_eq!(app.model_index, 1);

    app.on_key(key(KeyCode::Enter)); // 保存

    // 验证：model_id 改变、id 保持稳定
    let updated = app.dao.get_instance("minimax-cl-mini").unwrap();
    assert_eq!(updated.model_id, "MiniMax-M3");
    assert_eq!(updated.id, "minimax-cl-mini");
    assert_eq!(updated.alias, "cl-mini");
    assert_eq!(updated.api_key, "sk-test");
}

/// 改 model 后 opencode_model_id 自动重算（原值为空时）
#[test]
fn test_edit_model_auto_recomputes_opencode_id() {
    let templates = vec![minimax_template()];
    let dao = MemoryDaoImpl::new(templates);
    let mut app = App::new_with_dao(dao);

    let inst = ProviderInstance {
        id: "minimax-cl-mini".to_string(),
        template_id: "minimax".to_string(),
        model_id: "MiniMax-M2.7-highspeed".to_string(),
        api_key: "sk-test".to_string(),
        created_at: Utc::now(),
        alias: "cl-mini".to_string(),
        opencode_model_id: String::new(), // 空，应该自动重算
        kv_cache_enabled: false,
    };
    app.dao.create_instance(inst).unwrap();

    app.state = AppState::EditInfoPanel {
        instance_id: "minimax-cl-mini".to_string(),
        focus_index: 0,
    };
    app.on_key(key(KeyCode::Enter)); // 进 EditModel
    app.on_key(key(KeyCode::Down)); // 选 M3
    app.on_key(key(KeyCode::Enter)); // 保存

    let updated = app.dao.get_instance("minimax-cl-mini").unwrap();
    assert_eq!(updated.opencode_model_id, "MiniMax-M3");
}

/// 改 model 时如果 opencode_model_id 已设值，保留用户原值
#[test]
fn test_edit_model_preserves_user_opencode_id() {
    let templates = vec![minimax_template()];
    let dao = MemoryDaoImpl::new(templates);
    let mut app = App::new_with_dao(dao);

    let inst = ProviderInstance {
        id: "minimax-cl-mini".to_string(),
        template_id: "minimax".to_string(),
        model_id: "MiniMax-M2.7-highspeed".to_string(),
        api_key: "sk-test".to_string(),
        created_at: Utc::now(),
        alias: "cl-mini".to_string(),
        opencode_model_id: "user-custom-id".to_string(), // 已设值，应该保留
        kv_cache_enabled: false,
    };
    app.dao.create_instance(inst).unwrap();

    app.state = AppState::EditInfoPanel {
        instance_id: "minimax-cl-mini".to_string(),
        focus_index: 0,
    };
    app.on_key(key(KeyCode::Enter));
    app.on_key(key(KeyCode::Down));
    app.on_key(key(KeyCode::Enter));

    let updated = app.dao.get_instance("minimax-cl-mini").unwrap();
    assert_eq!(updated.model_id, "MiniMax-M3");
    assert_eq!(updated.opencode_model_id, "user-custom-id");
}
