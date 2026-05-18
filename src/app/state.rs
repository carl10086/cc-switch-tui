use crate::app::templates::register_templates;
use crate::dao::memory_impl::MemoryDaoImpl;
use crate::dao::Dao;
use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
use crate::opencode_fetch::OpencodeModelCache;
use std::collections::HashSet;

/// 应用当前所处的页面状态
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// 主界面：实例列表 + 信息面板
    List,
    /// 新建向导：选择 Provider
    CreateProvider,
    /// 新建向导：选择 Model
    CreateModel { template_id: String },
    /// 新建向导：输入 API Key
    CreateApiKey { template_id: String, model_id: String },
    /// 新建向导：选择 OpenCode Model
    CreateOpencodeModel { template_id: String, model_id: String, api_key: String },
    /// 新建向导最后一页：输入别名
    CreateAlias { template_id: String, model_id: String, api_key: String, opencode_model_id: String },
    /// 原有 Edit 保留给 API Key 弹窗（兼容现有 draw_edit）
    Edit { instance_id: String },
    /// 编辑右侧信息面板
    EditInfoPanel { instance_id: String, focus_index: usize },
    /// 编辑具体字段弹窗
    EditField { instance_id: String, field: EditField },
    /// 编辑 OpenCode Model（列表选择）
    EditOpencodeModel { instance_id: String },
    /// 删除确认对话框
    DeleteConfirm { instance_id: String },
}

/// 编辑字段类型
#[derive(Debug, Clone, PartialEq)]
pub enum EditField {
    Alias,
    ApiKey,
    KvCacheEnabled,
}

/// 输入框状态，用于 API Key 输入和编辑
#[derive(Debug, Clone, PartialEq)]
pub struct InputState {
    /// 当前输入的字符串
    pub value: String,
    /// 光标在字符串中的位置（字符索引）
    pub cursor: usize,
}

impl InputState {
    /// 使用默认值创建输入状态
    pub fn new(value: String) -> Self {
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    /// 在光标位置插入一个字符
    pub fn insert_char(&mut self, c: char) {
        let byte_pos = self.value.char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len());
        self.value.insert(byte_pos, c);
        self.cursor += 1;
    }

    /// 删除光标前一个字符
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_pos = self.value.char_indices()
            .nth(self.cursor - 1)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let next_byte_pos = self.value.char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len());
        self.value.drain(byte_pos..next_byte_pos);
        self.cursor -= 1;
    }

    /// 光标左移
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// 光标右移
    pub fn move_right(&mut self) {
        if self.cursor < self.value.chars().count() {
            self.cursor += 1;
        }
    }
}

/// 应用主结构体，包含 Dao、状态、输入、列表索引等
pub struct App<D: Dao> {
    /// 数据访问对象
    pub dao: D,
    /// 当前页面状态
    pub state: AppState,
    /// 实例列表中当前高亮的索引
    pub list_index: usize,
    /// 创建向导中 Provider 列表的索引
    pub provider_index: usize,
    /// 创建向导中 Model 列表的索引
    pub model_index: usize,
    /// 创建向导中 OpenCode Model 列表的索引
    pub opencode_model_index: usize,
    /// API Key 输入状态
    pub api_key_input: InputState,
    /// 编辑时的输入状态
    pub edit_input: InputState,
    /// 错误消息（显示在主界面底部）
    pub error_message: Option<String>,
    /// 是否退出应用
    pub should_quit: bool,
    /// ~/.zshrc 是否在本次启动时被自动修改
    pub zshrc_modified: bool,
    /// 从 models.dev/api.json 拉取并缓存的 OpenCode 模型列表
    pub opencode_model_cache: OpencodeModelCache,
}

impl App<MemoryDaoImpl> {
    /// 创建新的 App 实例
    pub fn new() -> Self {
        let templates = register_templates();
        Self::new_with_dao(MemoryDaoImpl::new(templates))
    }
}

impl<D: Dao> App<D> {
    pub fn new_with_dao(dao: D) -> Self {
        Self {
            dao,
            state: AppState::List,
            list_index: 0,
            provider_index: 0,
            model_index: 0,
            opencode_model_index: 0,
            api_key_input: InputState::new(String::new()),
            edit_input: InputState::new(String::new()),
            error_message: None,
            should_quit: false,
            zshrc_modified: false,
            opencode_model_cache: OpencodeModelCache::new(),
        }
    }

    /// 获取所有已创建的实例，按模板顺序分组排列
    ///
    /// 排序规则：按 templates 顺序 → 按 template.models 顺序 →
    /// 同 (template_id, model_id) 内按 created_at 升序
    pub fn get_sorted_instances(&self) -> Vec<&ProviderInstance> {
        let templates = self.dao.get_templates();
        let instances = self.dao.list_instances();
        let mut result = Vec::new();
        for template in templates {
            for model in &template.models {
                let mut group: Vec<&ProviderInstance> = instances.iter()
                    .filter(|i| i.template_id == template.id && i.model_id == model.id)
                    .copied()
                    .collect();
                group.sort_by_key(|i| i.created_at);
                result.extend(group);
            }
        }
        result
    }

    /// 获取当前高亮的实例
    pub fn current_instance(&self) -> Option<&ProviderInstance> {
        let instances = self.get_sorted_instances();
        instances.get(self.list_index).copied()
    }

    /// 获取当前选中的 Provider 模板（用于新建向导）
    pub fn current_provider(&self) -> Option<&ProviderTemplate> {
        let templates = self.dao.get_templates();
        templates.get(self.provider_index).copied()
    }

    /// 获取当前选中的 Model（用于新建向导）
    pub fn current_model(&self) -> Option<&crate::domain::ModelTemplate> {
        let provider = self.current_provider()?;
        provider.models.get(self.model_index)
    }

    /// 获取当前 Provider 下可用的 OpenCode Model ID 列表
    pub fn get_opencode_models_for_current_provider(&self) -> Vec<String> {
        let provider = match self.current_provider() {
            Some(p) => p,
            None => return vec![],
        };
        self.get_opencode_models_for_provider_id(&provider.id)
    }

    /// 获取指定 Provider ID 下可用的 OpenCode Model ID 列表
    /// 优先从内存缓存（models.dev/api.json）获取，合并模板硬编码的映射
    pub fn get_opencode_models_for_provider_id(&self, template_id: &str) -> Vec<String> {
        let provider = match self.dao.get_template(template_id) {
            Some(p) => p,
            None => return vec![],
        };

        let mut set: HashSet<String> = provider
            .models
            .iter()
            .map(|m| m.opencode_model_id.clone())
            .filter(|id| !id.is_empty())
            .collect();

        // 合并从 API 拉取并缓存的模型列表
        if !provider.opencode_provider_id.is_empty() {
            if let Some(cached) = self.opencode_model_cache.get(&provider.opencode_provider_id) {
                set.extend(cached.iter().cloned());
            }
        }

        set.into_iter().collect()
    }

    /// 重新生成 aliases.zsh（静默忽略错误）
    fn regenerate_aliases(&self) {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let _ = crate::shell::generate_aliases(
            &home.join(".cc-switch-tui"),
            &self.dao.list_instances().into_iter().cloned().collect::<Vec<_>>(),
            &self.dao.get_templates().into_iter().cloned().collect::<Vec<_>>(),
        );
    }
}

use crossterm::event::{KeyCode, KeyEvent};

impl<D: Dao> App<D> {
    /// 将索引向指定方向移动一位，限制在 [0, max) 范围内
    fn move_index(current: usize, delta: i32, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        let next = current as i32 + delta;
        next.clamp(0, max as i32 - 1) as usize
    }

    /// 处理键盘事件
    pub fn on_key(&mut self, key: KeyEvent) {
        self.error_message = None;
        tracing::debug!("key_event = {:?}", key);
        match &self.state.clone() {
            AppState::List => self.handle_list(key),
            AppState::CreateProvider => self.handle_create_provider(key),
            AppState::CreateModel { .. } => self.handle_create_model(key),
            AppState::CreateApiKey { .. } => self.handle_create_api_key(key),
            AppState::CreateOpencodeModel { .. } => self.handle_create_opencode_model(key),
            AppState::CreateAlias { .. } => self.handle_create_alias(key),
            AppState::Edit { .. } => self.handle_edit(key),
            AppState::EditInfoPanel { .. } => self.handle_edit_info_panel(key),
            AppState::EditField { .. } => self.handle_edit_field(key),
            AppState::EditOpencodeModel { .. } => self.handle_edit_opencode_model(key),
            AppState::DeleteConfirm { .. } => self.handle_delete_confirm(key),
        }
    }

    fn handle_list(&mut self, key: KeyEvent) {
        let instances = self.get_sorted_instances();
        match key.code {
            KeyCode::Char('q') => {
                tracing::debug!("state transition: List -> Quit");
                self.should_quit = true;
            }
            KeyCode::Char('n') => {
                tracing::debug!("state transition: List -> CreateProvider");
                self.state = AppState::CreateProvider;
                self.provider_index = 0;
            }
            KeyCode::Char('e') => {
                if let Some(instance) = self.current_instance() {
                    tracing::debug!("state transition: List -> EditInfoPanel({})", instance.id);
                    self.state = AppState::EditInfoPanel {
                        instance_id: instance.id.clone(),
                        focus_index: 0,
                    };
                }
            }
            KeyCode::Char('d') => {
                if let Some(instance) = self.current_instance() {
                    tracing::debug!("state transition: List -> DeleteConfirm({})", instance.id);
                    self.state = AppState::DeleteConfirm { instance_id: instance.id.clone() };
                }
            }
            KeyCode::Enter => {
                if let Some(instance) = self.current_instance() {
                    let alias = instance.alias.clone();
                    if alias.is_empty() {
                        self.error_message = Some("请先按 e 进入编辑模式设置别名".to_string());
                    } else {
                        self.regenerate_aliases();
                        self.error_message = Some(format!("已激活 {}，新终端中 claude 命令将使用该配置", alias));
                    }
                }
            }
            KeyCode::Up => self.list_index = Self::move_index(self.list_index, -1, instances.len()),
            KeyCode::Down => self.list_index = Self::move_index(self.list_index, 1, instances.len()),
            _ => {}
        }
    }

    fn handle_create_provider(&mut self, key: KeyEvent) {
        let templates = self.dao.get_templates();
        match key.code {
            KeyCode::Esc => self.state = AppState::List,
            KeyCode::Enter => {
                if let Some(template) = templates.get(self.provider_index) {
                    self.state = AppState::CreateModel {
                        template_id: template.id.clone(),
                    };
                    self.model_index = 0;
                }
            }
            KeyCode::Up => self.provider_index = Self::move_index(self.provider_index, -1, templates.len()),
            KeyCode::Down => self.provider_index = Self::move_index(self.provider_index, 1, templates.len()),
            _ => {}
        }
    }

    fn handle_create_model(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.state = AppState::CreateProvider,
            KeyCode::Enter => {
                if let Some(model) = self.current_model() {
                    if let Some(template) = self.current_provider() {
                        self.state = AppState::CreateApiKey {
                            template_id: template.id.clone(),
                            model_id: model.id.clone(),
                        };
                        self.api_key_input = InputState::new(String::new());
                    }
                }
            }
            KeyCode::Up => {
                if let Some(template) = self.current_provider() {
                    self.model_index = Self::move_index(self.model_index, -1, template.models.len());
                }
            }
            KeyCode::Down => {
                if let Some(template) = self.current_provider() {
                    self.model_index = Self::move_index(self.model_index, 1, template.models.len());
                }
            }
            _ => {}
        }
    }

    fn handle_create_api_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::CreateApiKey { template_id, .. } = &self.state {
                    let tid = template_id.clone();
                    self.state = AppState::CreateModel { template_id: tid };
                }
            }
            KeyCode::Enter => {
                if let AppState::CreateApiKey { template_id, model_id } = self.state.clone() {
                    let api_key = self.api_key_input.value.clone();
                    self.state = AppState::CreateOpencodeModel {
                        template_id,
                        model_id,
                        api_key,
                    };
                    self.opencode_model_index = 0;
                }
            }
            KeyCode::Backspace => self.api_key_input.backspace(),
            KeyCode::Left => self.api_key_input.move_left(),
            KeyCode::Right => self.api_key_input.move_right(),
            KeyCode::Char(c) => self.api_key_input.insert_char(c),
            _ => {}
        }
    }

    fn handle_create_opencode_model(&mut self, key: KeyEvent) {
        let models = self.get_opencode_models_for_current_provider();
        match key.code {
            KeyCode::Esc => {
                if let AppState::CreateOpencodeModel { template_id, model_id, api_key } = self.state.clone() {
                    self.state = AppState::CreateApiKey {
                        template_id,
                        model_id,
                    };
                    self.api_key_input = InputState::new(api_key);
                }
            }
            KeyCode::Enter => {
                if let AppState::CreateOpencodeModel { template_id, model_id, api_key } = self.state.clone() {
                    let opencode_model_id = models.get(self.opencode_model_index)
                        .map(|m| m.clone())
                        .unwrap_or_default();
                    self.state = AppState::CreateAlias {
                        template_id,
                        model_id,
                        api_key,
                        opencode_model_id,
                    };
                    self.edit_input = InputState::new(String::new());
                }
            }
            KeyCode::Up => self.opencode_model_index = Self::move_index(self.opencode_model_index, -1, models.len()),
            KeyCode::Down => self.opencode_model_index = Self::move_index(self.opencode_model_index, 1, models.len()),
            _ => {}
        }
    }

    fn handle_create_alias(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::CreateAlias { template_id, model_id, api_key, .. } = self.state.clone() {
                    self.state = AppState::CreateOpencodeModel {
                        template_id,
                        model_id,
                        api_key,
                    };
                }
            }
            KeyCode::Enter => {
                self.submit_create();
            }
            KeyCode::Backspace => self.edit_input.backspace(),
            KeyCode::Left => self.edit_input.move_left(),
            KeyCode::Right => self.edit_input.move_right(),
            KeyCode::Char(c) => self.edit_input.insert_char(c),
            _ => {}
        }
    }

    fn submit_create(&mut self) {
        if let AppState::CreateAlias { template_id, model_id, api_key, opencode_model_id } = self.state.clone() {
            let alias = self.edit_input.value.clone();
            if let Err(e) = self.validate_alias(&alias) {
                self.error_message = Some(e.to_string());
                return;
            }
            let id = format!("{}-{}-{}", template_id, model_id, alias);
            let instance = ProviderInstance {
                id: id.clone(),
                template_id,
                model_id,
                api_key,
                created_at: chrono::Utc::now(),
                alias,
                opencode_model_id,
                kv_cache_enabled: false,
            };
            match self.dao.create_instance(instance) {
                Ok(()) => {
                    tracing::info!("create instance success: id={}", id);
                    self.regenerate_aliases();
                    self.state = AppState::List;
                    self.list_index = self.get_sorted_instances().len().saturating_sub(1);
                }
                Err(AppError::InstanceAlreadyExists(id)) => {
                    self.error_message = Some(format!("实例已存在: {}", id));
                    self.state = AppState::List;
                }
                Err(e) => {
                    self.error_message = Some(e.to_string());
                    self.state = AppState::List;
                }
            }
        }
    }

    fn validate_alias(&self, alias: &str) -> Result<(), AppError> {
        if alias.is_empty() {
            return Err(AppError::InvalidAlias("alias cannot be empty".to_string()));
        }
        if !alias.starts_with("cl-") {
            return Err(AppError::InvalidAlias("alias must start with 'cl-'".to_string()));
        }
        if !alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(AppError::InvalidAlias("alias contains invalid characters".to_string()));
        }
        let instances = self.dao.list_instances();
        if instances.iter().any(|i| i.alias == alias) {
            return Err(AppError::AliasAlreadyExists(alias.to_string()));
        }
        Ok(())
    }

    fn handle_edit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.state = AppState::List,
            KeyCode::Enter => {
                if let AppState::Edit { instance_id } = self.state.clone() {
                    if let Err(e) = self.dao.update_instance(&instance_id, self.edit_input.value.clone()) {
                        self.error_message = Some(e.to_string());
                    } else {
                        tracing::info!("update api_key: id={}", instance_id);
                    }
                    self.state = AppState::List;
                }
            }
            KeyCode::Backspace => self.edit_input.backspace(),
            KeyCode::Left => self.edit_input.move_left(),
            KeyCode::Right => self.edit_input.move_right(),
            KeyCode::Char(c) => self.edit_input.insert_char(c),
            _ => {}
        }
    }

    fn handle_edit_info_panel(&mut self, key: KeyEvent) {
        let max_index = 3; // alias=0, api_key=1, opencode_model=2, kv_cache=3
        match key.code {
            KeyCode::Esc => self.state = AppState::List,
            KeyCode::Up => {
                if let AppState::EditInfoPanel { instance_id, focus_index } = self.state.clone() {
                    if focus_index > 0 {
                        self.state = AppState::EditInfoPanel {
                            instance_id,
                            focus_index: focus_index - 1,
                        };
                    }
                }
            }
            KeyCode::Down => {
                if let AppState::EditInfoPanel { instance_id, focus_index } = self.state.clone() {
                    if focus_index < max_index {
                        self.state = AppState::EditInfoPanel {
                            instance_id,
                            focus_index: focus_index + 1,
                        };
                    }
                }
            }
            KeyCode::Enter => {
                if let AppState::EditInfoPanel { instance_id, focus_index } = self.state.clone() {
                    match focus_index {
                        0 | 1 => {
                            let field = match focus_index {
                                0 => EditField::Alias,
                                1 => EditField::ApiKey,
                                _ => return,
                            };
                            if let Some(instance) = self.dao.get_instance(&instance_id) {
                                let value = match field {
                                    EditField::Alias => instance.alias.clone(),
                                    EditField::ApiKey => instance.api_key.clone(),
                                    EditField::KvCacheEnabled => instance.kv_cache_enabled.to_string(),
                                };
                                self.edit_input = InputState::new(value);
                            }
                            self.state = AppState::EditField { instance_id, field };
                        }
                        2 => {
                            // OpenCode Model 使用列表选择而非文本输入
                            if let Some(instance) = self.dao.get_instance(&instance_id) {
                                let models = self.get_opencode_models_for_provider_id(&instance.template_id);
                                let current_index = models
                                    .iter()
                                    .position(|m| m == &instance.opencode_model_id)
                                    .unwrap_or(0);
                                self.opencode_model_index = current_index;
                            }
                            self.state = AppState::EditOpencodeModel { instance_id };
                        }
                        3 => {
                            // KV Cache 开关：直接切换布尔值
                            if let Some(instance) = self.dao.get_instance(&instance_id) {
                                let new_enabled = !instance.kv_cache_enabled;
                                if let Err(e) = self.dao.set_kv_cache_enabled(&instance_id, new_enabled) {
                                    self.error_message = Some(e.to_string());
                                } else {
                                    tracing::info!("toggle kv_cache_enabled: id={}, enabled={}", instance_id, new_enabled);
                                    self.regenerate_aliases();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_edit_field(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::EditField { instance_id, field } = self.state.clone() {
                    let focus_index = match field {
                        EditField::Alias => 0,
                        EditField::ApiKey => 1,
                        EditField::KvCacheEnabled => 3,
                    };
                    self.state = AppState::EditInfoPanel {
                        instance_id,
                        focus_index,
                    };
                }
            }
            KeyCode::Enter => {
                if let AppState::EditField { instance_id, field } = self.state.clone() {
                    let value = self.edit_input.value.clone();
                    let (result, new_instance_id) = match field {
                        EditField::Alias => {
                            if let Err(e) = self.validate_alias(&value) {
                                (Err(e), instance_id.clone())
                            } else {
                                // Get old instance to compute new id
                                let old_instance = match self.dao.get_instance(&instance_id) {
                                    Some(i) => i,
                                    None => return,
                                };
                                let new_id = format!("{}-{}-{}", old_instance.template_id, old_instance.model_id, value);
                                let result = self.dao.rename_instance(&instance_id, &new_id, value);
                                (result, new_id)
                            }
                        }
                        EditField::ApiKey => {
                            (self.dao.update_instance(&instance_id, value), instance_id.clone())
                        }
                        EditField::KvCacheEnabled => {
                            // This case is handled in handle_edit_info_panel, not here
                            (Ok(()), instance_id.clone())
                        }
                    };
                    match result {
                        Ok(()) => {
                            self.regenerate_aliases();
                            self.state = AppState::EditInfoPanel {
                                instance_id: new_instance_id,
                                focus_index: 0,
                            };
                        }
                        Err(e) => {
                            self.error_message = Some(e.to_string());
                        }
                    }
                }
            }
            KeyCode::Backspace => self.edit_input.backspace(),
            KeyCode::Left => self.edit_input.move_left(),
            KeyCode::Right => self.edit_input.move_right(),
            KeyCode::Char(c) => self.edit_input.insert_char(c),
            _ => {}
        }
    }

    fn handle_edit_opencode_model(&mut self, key: KeyEvent) {
        let models = if let AppState::EditOpencodeModel { instance_id } = &self.state {
            self.dao
                .get_instance(instance_id)
                .map(|i| self.get_opencode_models_for_provider_id(&i.template_id))
                .unwrap_or_default()
        } else {
            vec![]
        };
        match key.code {
            KeyCode::Esc => {
                if let AppState::EditOpencodeModel { instance_id } = self.state.clone() {
                    self.state = AppState::EditInfoPanel {
                        instance_id,
                        focus_index: 2,
                    };
                }
            }
            KeyCode::Enter => {
                if let AppState::EditOpencodeModel { instance_id } = self.state.clone() {
                    let opencode_model_id = models
                        .get(self.opencode_model_index)
                        .cloned()
                        .unwrap_or_default();
                    if let Err(e) = self.dao.set_opencode_model_id(&instance_id, opencode_model_id) {
                        self.error_message = Some(e.to_string());
                    } else {
                        self.regenerate_aliases();
                    }
                    self.state = AppState::EditInfoPanel {
                        instance_id,
                        focus_index: 2,
                    };
                }
            }
            KeyCode::Up => self.opencode_model_index = Self::move_index(self.opencode_model_index, -1, models.len()),
            KeyCode::Down => self.opencode_model_index = Self::move_index(self.opencode_model_index, 1, models.len()),
            _ => {}
        }
    }

    fn handle_delete_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let AppState::DeleteConfirm { instance_id } = self.state.clone() {
                    if let Err(e) = self.dao.delete_instance(&instance_id) {
                        self.error_message = Some(e.to_string());
                    } else {
                        tracing::info!("delete instance: id={}", instance_id);
                        self.regenerate_aliases();
                        let instances = self.get_sorted_instances();
                        if self.list_index >= instances.len() && self.list_index > 0 {
                            self.list_index -= 1;
                        }
                    }
                    self.state = AppState::List;
                }
            }
            _ => self.state = AppState::List,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ModelTemplate, ProviderTemplate};
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
                    opencode_model_id: "m1".to_string(),
                }],
                opencode_provider_id: "minimax-cn".to_string(),
                opencode_npm: "@ai-sdk/anthropic".to_string(),
                opencode_base_url: "https://api.minimaxi.com/anthropic/v1".to_string(),
                opencode_env_var: "MINIMAX_API_KEY".to_string(),
            },
            ProviderTemplate {
                id: "kimi".to_string(),
                name: "Kimi".to_string(),
                default_env: HashMap::new(),
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
            },
        ]
    }

    #[test]
    fn test_get_sorted_instances_groups_by_template_then_created_at() {
        let dao = MemoryDaoImpl::new(test_templates());
        let mut app = App::new_with_dao(dao);

        let i1 = ProviderInstance {
            id: "kimi-kimi-for-coding-cl-km2".to_string(),
            template_id: "kimi".to_string(),
            model_id: "kimi-for-coding".to_string(),
            api_key: "key1".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::seconds(10),
            alias: "cl-km2".to_string(),
            opencode_model_id: "k2p5".to_string(),
            kv_cache_enabled: false,
        };
        let i2 = ProviderInstance {
            id: "kimi-kimi-for-coding-cl-km3".to_string(),
            template_id: "kimi".to_string(),
            model_id: "kimi-for-coding".to_string(),
            api_key: "key2".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-km3".to_string(),
            opencode_model_id: "k2p5".to_string(),
            kv_cache_enabled: false,
        };
        let i3 = ProviderInstance {
            id: "minimax-m1-cl-mx".to_string(),
            template_id: "minimax".to_string(),
            model_id: "m1".to_string(),
            api_key: "key3".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mx".to_string(),
            opencode_model_id: "m1".to_string(),
            kv_cache_enabled: false,
        };

        app.dao.create_instance(i1.clone()).unwrap();
        app.dao.create_instance(i2.clone()).unwrap();
        app.dao.create_instance(i3.clone()).unwrap();

        let sorted = app.get_sorted_instances();
        assert_eq!(sorted.len(), 3, "应返回全部 3 个实例");
        assert_eq!(sorted[0].template_id, "minimax", "template 顺序优先");
        assert_eq!(sorted[1].template_id, "kimi");
        assert_eq!(sorted[2].template_id, "kimi");
        assert_eq!(
            sorted[1].created_at, i1.created_at,
            "同组内按 created_at 升序，i1 更早应在前面"
        );
        assert_eq!(sorted[2].created_at, i2.created_at);
    }

    #[test]
    fn test_get_sorted_instances_empty_when_no_instances() {
        let dao = MemoryDaoImpl::new(test_templates());
        let app = App::new_with_dao(dao);
        let sorted = app.get_sorted_instances();
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_get_sorted_instances_handles_multiple_aliases_same_model() {
        let dao = MemoryDaoImpl::new(test_templates());
        let mut app = App::new_with_dao(dao);

        let i1 = ProviderInstance {
            id: "kimi-kimi-for-coding-cl-km2".to_string(),
            template_id: "kimi".to_string(),
            model_id: "kimi-for-coding".to_string(),
            api_key: "key1".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-km2".to_string(),
            opencode_model_id: "k2p5".to_string(),
            kv_cache_enabled: false,
        };
        let i2 = ProviderInstance {
            id: "kimi-kimi-for-coding-cl-km3".to_string(),
            template_id: "kimi".to_string(),
            model_id: "kimi-for-coding".to_string(),
            api_key: "key2".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::seconds(5),
            alias: "cl-km3".to_string(),
            opencode_model_id: "k2p5".to_string(),
            kv_cache_enabled: false,
        };

        app.dao.create_instance(i1.clone()).unwrap();
        app.dao.create_instance(i2.clone()).unwrap();

        let sorted = app.get_sorted_instances();
        assert_eq!(
            sorted.len(),
            2,
            "同一 model 下多个 alias 实例应全部被返回"
        );
        assert_eq!(sorted[0].id, i2.id, "i2 创建更早应在前面");
        assert_eq!(sorted[1].id, i1.id);
    }
}
