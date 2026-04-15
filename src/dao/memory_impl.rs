use crate::dao::Dao;
use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
use std::collections::HashMap;

/// 基于内存的 Dao 实现，数据仅在进程生命周期内有效
pub struct MemoryDaoImpl {
    /// 内置 Provider 模板列表
    templates: Vec<ProviderTemplate>,
    /// 用户创建的实例映射，key 为 instance.id
    instances: HashMap<String, ProviderInstance>,
    /// 当前选中的实例 ID
    current_instance_id: Option<String>,
}

impl MemoryDaoImpl {
    /// 使用给定的模板列表创建新的 MemoryDaoImpl
    pub fn new(templates: Vec<ProviderTemplate>) -> Self {
        Self {
            templates,
            instances: HashMap::new(),
            current_instance_id: None,
        }
    }
}

impl Dao for MemoryDaoImpl {
    fn get_templates(&self) -> Vec<&ProviderTemplate> {
        self.templates.iter().collect()
    }

    fn get_template(&self, id: &str) -> Option<&ProviderTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    fn list_instances(&self) -> Vec<&ProviderInstance> {
        self.instances.values().collect()
    }

    fn get_instance(&self, id: &str) -> Option<&ProviderInstance> {
        self.instances.get(id)
    }

    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
        if self.instances.contains_key(&instance.id) {
            return Err(AppError::InstanceAlreadyExists(instance.id.clone()));
        }
        let id = instance.id.clone();
        self.instances.insert(id.clone(), instance);
        tracing::info!("dao create_instance: id={}", id);
        Ok(())
    }

    fn delete_instance(&mut self, id: &str) -> Result<(), AppError> {
        if self.instances.remove(id).is_none() {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        tracing::info!("dao delete_instance: id={}", id);
        if self.current_instance_id.as_deref() == Some(id) {
            self.current_instance_id = None;
        }
        Ok(())
    }

    fn get_current_instance(&self) -> Option<&ProviderInstance> {
        self.current_instance_id
            .as_ref()
            .and_then(|id| self.instances.get(id))
    }

    fn set_current_instance(&mut self, id: &str) -> Result<(), AppError> {
        if !self.instances.contains_key(id) {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.current_instance_id = Some(id.to_string());
        Ok(())
    }

    fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError> {
        let instance = self.instances.get_mut(id)
            .ok_or_else(|| AppError::InstanceNotFound(id.to_string()))?;
        instance.api_key = api_key;
        tracing::info!("dao update_instance: id={}", id);
        Ok(())
    }

    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError> {
        let instance = self.instances.get_mut(id)
            .ok_or_else(|| AppError::InstanceNotFound(id.to_string()))?;
        instance.alias = alias;
        Ok(())
    }
}
