use crate::domain::{ProviderInstance, ProviderTemplate};
use crate::shell;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// 单个 instance 的 opencode 配置 JSON（不写盘）。
/// 返回 None 表示该 instance 不应该生成 opencode 配置（缺字段等）。
pub fn render_opencode_config(
    instance: &ProviderInstance,
    template: &ProviderTemplate,
) -> Option<Value> {
    if instance.alias.is_empty() {
        return None;
    }
    let model_template = template.models.iter().find(|m| m.id == instance.model_id);
    let opencode_model_id = if instance.opencode_model_id.is_empty() {
        model_template
            .map(|m| m.opencode_model_id.clone())
            .unwrap_or_default()
    } else {
        instance.opencode_model_id.clone()
    };
    if template.opencode_provider_id.is_empty() || opencode_model_id.is_empty() {
        return None;
    }
    let model_name = model_template
        .map(|m| m.name.clone())
        .unwrap_or_else(|| opencode_model_id.clone());

    Some(json!({
        "$schema": "https://opencode.ai/config.json",
        "model": format!("{}/{}", template.opencode_provider_id, opencode_model_id),
        "provider": {
            &template.opencode_provider_id: {
                "npm": &template.opencode_npm,
                "name": &template.name,
                "options": {
                    "baseURL": &template.opencode_base_url,
                    "apiKey": format!("{{env:{}}}", &template.opencode_env_var)
                },
                "models": {
                    &opencode_model_id: {
                        "name": model_name
                    }
                }
            }
        }
    }))
}

/// 配置文件的实际路径：`{dir}/opencode/{alias}.json`
pub fn opencode_config_path(dir: &Path, alias: &str) -> PathBuf {
    dir.join("opencode").join(format!("{alias}.json"))
}

/// 写入单个 instance 的配置到 `{dir}/opencode/{alias}.json`。
/// 权限设为 600（仅 owner 可读写）。返回 Some(path) 成功，None 表示跳过。
pub fn write_opencode_config(
    dir: &Path,
    instance: &ProviderInstance,
    template: &ProviderTemplate,
) -> std::io::Result<Option<PathBuf>> {
    let Some(config) = render_opencode_config(instance, template) else {
        return Ok(None);
    };
    let path = opencode_config_path(dir, &instance.alias);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_str = serde_json::to_string_pretty(&config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(&path, json_str)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok(Some(path))
}

/// 生成所有 opencode 配置文件，返回每个 alias 对应的配置文件路径。
/// 保留向后兼容 — main.rs / shell::generate_aliases 都调用此函数。
pub fn generate_opencode_configs(
    dir: &Path,
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
) -> std::io::Result<HashMap<String, PathBuf>> {
    fs::create_dir_all(dir.join("opencode"))?;

    let template_map: HashMap<&str, &ProviderTemplate> =
        templates.iter().map(|t| (t.id.as_str(), t)).collect();

    let mut result = HashMap::new();
    for instance in instances {
        if instance.alias.is_empty() {
            continue;
        }
        let Some(template) = template_map.get(instance.template_id.as_str()) else {
            continue;
        };
        if let Some(path) = write_opencode_config(dir, instance, template)? {
            result.insert(instance.alias.clone(), path);
        }
    }
    Ok(result)
}

/// 收集所有需要 unset 的 opencode 环境变量（来自所有模板）
fn get_all_opencode_env_vars(templates: &[ProviderTemplate]) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::from(["OPENCODE_CONFIG".to_string()]);
    for template in templates {
        if !template.opencode_env_var.is_empty() {
            set.insert(template.opencode_env_var.clone());
        }
    }
    let mut vars: Vec<String> = set.into_iter().collect();
    vars.sort();
    vars
}

/// 构建 opencode alias 函数定义
pub fn build_opencode_aliases(
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
    config_paths: &HashMap<String, PathBuf>,
) -> Vec<String> {
    let mut lines = vec![];

    // 用 HashMap 索引模板，避免每次线性查找
    let template_map: HashMap<&str, &ProviderTemplate> =
        templates.iter().map(|t| (t.id.as_str(), t)).collect();

    // 收集所有需要 unset 的环境变量，避免切换到其他 provider 时残留上一个 provider 的 key
    let all_env_vars = get_all_opencode_env_vars(templates);

    for instance in instances {
        if instance.alias.is_empty() {
            continue;
        }
        let Some(template) = template_map.get(instance.template_id.as_str()) else {
            continue;
        };
        if template.opencode_provider_id.is_empty() {
            continue;
        }
        let Some(config_path) = config_paths.get(&instance.alias) else {
            continue;
        };

        let suffix = instance
            .alias
            .strip_prefix("cl-")
            .unwrap_or(&instance.alias);
        let opencode_alias = format!("oc-{}", suffix);
        let env_var = &template.opencode_env_var;

        let unset_line = format!("  unset {}", all_env_vars.join(" "));

        let function_def = format!(
            "function {} {{\n{}\n  export OPENCODE_CONFIG={}\n  export {}={}\n  command opencode \"$@\"\n}}",
            opencode_alias,
            unset_line,
            shell::shell_escape(config_path.to_string_lossy().as_ref()),
            env_var,
            shell::shell_escape(&instance.api_key)
        );
        lines.push(function_def);
    }

    lines
}
