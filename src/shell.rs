use crate::domain::{ProviderInstance, ProviderTemplate};

/// 将选中的实例配置应用到当前环境
/// TODO: 当前为空实现，后续将生成 source 脚本或设置环境变量
pub fn apply_config(instance: &ProviderInstance, template: &ProviderTemplate) {
    let _ = instance;
    let _ = template;
    // TODO: 生成 ~/.cc-switch-tui/env.sh
}
