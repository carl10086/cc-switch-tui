use crate::app::templates::register_templates;
use crate::dao::memory_impl::MemoryDaoImpl;
use crate::dao::Dao;
use crate::domain::{AppError, ProviderInstance, ProviderTemplate};

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
    /// 编辑页面
    Edit { instance_id: String },
    /// 删除确认对话框
    DeleteConfirm { instance_id: String },
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
    /// API Key 输入状态
    pub api_key_input: InputState,
    /// 编辑时的输入状态
    pub edit_input: InputState,
    /// 错误消息（显示在主界面底部）
    pub error_message: Option<String>,
    /// 是否退出应用
    pub should_quit: bool,
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
            api_key_input: InputState::new(String::new()),
            edit_input: InputState::new(String::new()),
            error_message: None,
            should_quit: false,
        }
    }

    /// 获取所有已创建的实例，按模板顺序分组排列
    pub fn get_sorted_instances(&self) -> Vec<&ProviderInstance> {
        let templates = self.dao.get_templates();
        let mut result = Vec::new();
        for template in templates {
            for model in &template.models {
                let id = format!("{}-{}", template.id, model.id);
                if let Some(instance) = self.dao.get_instance(&id) {
                    result.push(instance);
                }
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
}

use crossterm::event::{KeyCode, KeyEvent};

impl<D: Dao> App<D> {
    /// 处理键盘事件
    pub fn on_key(&mut self, key: KeyEvent) {
        self.error_message = None;
        tracing::debug!("key_event = {:?}", key);
        match &self.state.clone() {
            AppState::List => self.handle_list(key),
            AppState::CreateProvider => self.handle_create_provider(key),
            AppState::CreateModel { .. } => self.handle_create_model(key),
            AppState::CreateApiKey { .. } => self.handle_create_api_key(key),
            AppState::Edit { .. } => self.handle_edit(key),
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
                    tracing::debug!("state transition: List -> Edit({})", instance.id);
                    let api_key = instance.api_key.clone();
                    let instance_id = instance.id.clone();
                    self.edit_input = InputState::new(api_key);
                    self.state = AppState::Edit { instance_id };
                }
            }
            KeyCode::Char('d') => {
                if let Some(instance) = self.current_instance() {
                    tracing::debug!("state transition: List -> DeleteConfirm({})", instance.id);
                    self.state = AppState::DeleteConfirm { instance_id: instance.id.clone() };
                }
            }
            KeyCode::Up => {
                if self.list_index > 0 {
                    self.list_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.list_index + 1 < instances.len() {
                    self.list_index += 1;
                }
            }
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
            KeyCode::Up => {
                if self.provider_index > 0 {
                    self.provider_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.provider_index + 1 < templates.len() {
                    self.provider_index += 1;
                }
            }
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
                if self.current_provider().is_some() && self.model_index > 0 {
                    self.model_index -= 1;
                }
            }
            KeyCode::Down => {
                if let Some(template) = self.current_provider() {
                    if self.model_index + 1 < template.models.len() {
                        self.model_index += 1;
                    }
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
                self.submit_create();
            }
            KeyCode::Backspace => self.api_key_input.backspace(),
            KeyCode::Left => self.api_key_input.move_left(),
            KeyCode::Right => self.api_key_input.move_right(),
            KeyCode::Char(c) => self.api_key_input.insert_char(c),
            _ => {}
        }
    }

    fn submit_create(&mut self) {
        if let AppState::CreateApiKey { template_id, model_id } = self.state.clone() {
            let id = format!("{}-{}", template_id, model_id);
            let instance = ProviderInstance {
                id: id.clone(),
                template_id,
                model_id,
                api_key: self.api_key_input.value.clone(),
                created_at: chrono::Utc::now(),
            };
            match self.dao.create_instance(instance) {
                Ok(()) => {
                    tracing::info!("create instance success: id={}", id);
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

    fn handle_delete_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let AppState::DeleteConfirm { instance_id } = self.state.clone() {
                    if let Err(e) = self.dao.delete_instance(&instance_id) {
                        self.error_message = Some(e.to_string());
                    } else {
                        tracing::info!("delete instance: id={}", instance_id);
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
