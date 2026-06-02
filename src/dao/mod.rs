pub mod memory_impl;
pub mod sqlite_impl;

pub use self::memory_impl::MemoryDaoImpl;
pub use self::sqlite_impl::SqliteDaoImpl;

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

    /// 更新实例的 model_id、alias、api_key，如果实例不存在则返回错误
    /// （注：id 不在签名中——id 与 model_id 解耦后，原地改 model 不会触发主键变更）
    fn update_instance(
        &mut self,
        id: &str,
        model_id: String,
        alias: String,
        api_key: String,
    ) -> Result<(), AppError>;

    /// 更新实例别名
    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError>;

    /// 重命名实例（同时更新 id 和 alias）
    fn rename_instance(
        &mut self,
        old_id: &str,
        new_id: &str,
        alias: String,
    ) -> Result<(), AppError>;

    /// 更新实例的 OpenCode Model ID
    fn set_opencode_model_id(
        &mut self,
        id: &str,
        opencode_model_id: String,
    ) -> Result<(), AppError>;

    /// 更新实例的 KV Cache 优化开关
    fn set_kv_cache_enabled(&mut self, id: &str, enabled: bool) -> Result<(), AppError>;

    /// 更新实例的扩展上下文窗口开关
    fn set_context_window_enabled(&mut self, id: &str, enabled: bool) -> Result<(), AppError>;
}
