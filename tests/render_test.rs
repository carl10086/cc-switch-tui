use cc_switch_tui::app::state::{App, AppState, EditField};
use cc_switch_tui::dao::Dao;
use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;
use cc_switch_tui::domain::{ModelTemplate, ProviderInstance, ProviderTemplate};
use cc_switch_tui::ui;
use chrono::Utc;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
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

/// 渲染一帧并把 buffer 序列化为字符串，便于断言文本内容
fn render_to_string<D: Dao>(app: &App<D>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.get(x, y).symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 文档化测试：记录 ratatui 0.26 TestBackend 对 CJK 字符的渲染行为。
///
/// 在 ratatui 0.26 中，`Paragraph`/`Block::title` 对 CJK 字符按 width-2 计算列宽，
/// 每个 CJK 字符在 buffer 中占 2 个 cell（cell 1 是字符，cell 2 是 padding space）。
/// 因此 `symbol().to_string()` 收集出来的字符串在 CJK 字符之间有真实空格：
///   "选择 Model"  →  "选 择  Model"
///
/// 后续的 render_test 断言不能用连续 CJK substring（如 "选择 Model"），
/// 必须用 ASCII 关键字（如 "Model - MiniMax"）或单 CJK 字符（"选"）匹配。
#[test]
fn cjk_in_buffer_has_padding_spaces() {
    let dao = MemoryDaoImpl::new(vec![minimax_template()]);
    let mut app = App::new_with_dao(dao);
    app.state = AppState::CreateModel {
        template_id: "minimax".to_string(),
    };

    let rendered = render_to_string(&app, 80, 24);

    // 每个 CJK 字符后面跟着一个 padding space，弹出连续 CJK substring 是不可能的
    assert!(
        rendered.contains("Model - MiniMax"),
        "ASCII 关键字应能匹配"
    );
    // 单个 CJK 字符的 substring 是匹配的（"选" 后面是 padding " "，所以是 "选 "）
    assert!(
        rendered.contains("选 "),
        "单 CJK 字符加 padding 空格应能匹配"
    );
    // 连续 CJK substring 不匹配（"选择" 之间被 padding space 分开）
    assert!(
        !rendered.contains("选择"),
        "ratatui 0.26 TestBackend 不应有连续 CJK substring（padding 行为）"
    );
}

/// 回归测试：进入 EditModel 状态后必须渲染"选择 Model"popup
/// 防止 src/ui/create.rs 的 draw_create 漏写 EditModel 分支（PR #14 的 bug）
#[test]
fn edit_model_state_renders_model_select_popup() {
    let dao = MemoryDaoImpl::new(vec![minimax_template()]);
    let mut app = App::new_with_dao(dao);
    let inst = ProviderInstance {
        id: "minimax-cl-mini".to_string(),
        template_id: "minimax".to_string(),
        model_id: "MiniMax-M3".to_string(),
        api_key: "sk-test".to_string(),
        created_at: Utc::now(),
        alias: "cl-mini".to_string(),
        opencode_model_id: String::new(),
        kv_cache_enabled: false,
    };
    app.dao.create_instance(inst).unwrap();
    app.state = AppState::EditModel {
        instance_id: "minimax-cl-mini".to_string(),
    };

    let rendered = render_to_string(&app, 80, 24);

    // 标题里出现 "Model - " 模板名（ratatui 0.26 TestBackend 把 CJK 字符按 width=2 切 cell，
    // CJK 字符间会插入 padding space，所以用 ASCII 关键字断言最稳）
    assert!(
        rendered.contains("Model - MiniMax"),
        "EditModel popup 标题 'Model - MiniMax' 未渲染。完整输出：\n{}",
        rendered
    );
    // 单独断言 "选" 和 "择" 出现（紧贴 padding space），保护"选择"二字语义
    assert!(
        rendered.contains("选 ") && rendered.contains("择 "),
        "EditModel popup 标题 '选择' 字符缺失。完整输出：\n{}",
        rendered
    );
    assert!(
        rendered.contains("MiniMax M3"),
        "model 'MiniMax M3' 未出现在 popup 中。完整输出：\n{}",
        rendered
    );
    assert!(
        rendered.contains("MiniMax M2.7 Highspeed"),
        "model 'MiniMax M2.7 Highspeed' 未出现在 popup 中。完整输出：\n{}",
        rendered
    );
}

/// 回归测试：CreateModel 状态仍走 current_provider() 路径（不被 EditModel 改动破坏）
#[test]
fn create_model_state_renders_model_select_popup() {
    let dao = MemoryDaoImpl::new(vec![minimax_template()]);
    let mut app = App::new_with_dao(dao);
    app.state = AppState::CreateModel {
        template_id: "minimax".to_string(),
    };

    let rendered = render_to_string(&app, 80, 24);

    assert!(
        rendered.contains("Model - MiniMax"),
        "CreateModel popup 标题未渲染"
    );
    assert!(rendered.contains("MiniMax M3"));
    assert!(rendered.contains("MiniMax M2.7 Highspeed"));
}

/// 回归测试：EditOpencodeModel 状态必须渲染 OpenCode model 列表 popup
#[test]
fn edit_opencode_model_state_renders_popup() {
    let dao = MemoryDaoImpl::new(vec![minimax_template()]);
    let mut app = App::new_with_dao(dao);
    let inst = ProviderInstance {
        id: "minimax-cl-mini".to_string(),
        template_id: "minimax".to_string(),
        model_id: "MiniMax-M3".to_string(),
        api_key: "sk-test".to_string(),
        created_at: Utc::now(),
        alias: "cl-mini".to_string(),
        opencode_model_id: String::new(),
        kv_cache_enabled: false,
    };
    app.dao.create_instance(inst).unwrap();
    app.state = AppState::EditOpencodeModel {
        instance_id: "minimax-cl-mini".to_string(),
    };

    let rendered = render_to_string(&app, 80, 24);

    assert!(
        rendered.contains("OpenCode Model"),
        "EditOpencodeModel popup 标题未渲染。完整输出：\n{}",
        rendered
    );
    // hardcoded opencode_model_id from template
    assert!(
        rendered.contains("MiniMax-M3"),
        "opencode model id 'MiniMax-M3' 未出现在 popup。完整输出：\n{}",
        rendered
    );
}

/// 回归测试：EditField::Alias 状态必须渲染"修改别名"弹窗
#[test]
fn edit_field_alias_state_renders_alias_prompt() {
    let dao = MemoryDaoImpl::new(vec![minimax_template()]);
    let mut app = App::new_with_dao(dao);
    let inst = ProviderInstance {
        id: "minimax-cl-mini".to_string(),
        template_id: "minimax".to_string(),
        model_id: "MiniMax-M3".to_string(),
        api_key: "sk-test".to_string(),
        created_at: Utc::now(),
        alias: "cl-mini".to_string(),
        opencode_model_id: String::new(),
        kv_cache_enabled: false,
    };
    app.dao.create_instance(inst).unwrap();
    app.state = AppState::EditField {
        instance_id: "minimax-cl-mini".to_string(),
        field: EditField::Alias,
    };

    let rendered = render_to_string(&app, 80, 24);

    // "Alias" 关键字 + 单独断言 "修" 和 "改" 字符出现，保护"修改"二字语义
    assert!(
        rendered.contains("Alias"),
        "EditField::Alias 弹窗 'Alias' 关键字未渲染。完整输出：\n{}",
        rendered
    );
    assert!(
        rendered.contains("修 ") && rendered.contains("改 "),
        "EditField::Alias 弹窗 '修改' 字符缺失。完整输出：\n{}",
        rendered
    );
}
