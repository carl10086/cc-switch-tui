pub mod memory_impl;

use crate::domain::{AppError, ProviderInstance, ProviderTemplate};

/// 数据访问对象接口，抽象 provider 配置和实例的存储
pub trait Dao {
    /// 获取所有内置 Provider 模板
    fn get_templates(&self) -> Vec<&ProviderTemplate>;

    /// 根据 ID 获取 Provider 模板
    fn get_template(&self, id: &str) -> Option<&ProviderTemplate>;

    /// 获取所有用户创建的实例
    fn list_instances(&self) -> Vec<&ProviderInstance>;

    /// 根据 ID 获取实例
    fn get_instance(&self, id: &str) -> Option<&ProviderInstance>;

    /// 创建实例，如果实例已存在则返回错误
    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError>;

    /// 删除实例，如果实例不存在则返回错误
    fn delete_instance(&mut self, id: &str) -> Result<(), AppError>;

    /// 获取当前选中的实例
    fn get_current_instance(&self) -> Option<&ProviderInstance>;

    /// 设置当前选中的实例
    fn set_current_instance(&mut self, id: &str) -> Result<(), AppError>;
}
