use crate::domain::{ProviderInstance, ProviderTemplate};
use crate::shell;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// 生成所有 opencode 配置文件，返回每个 alias 对应的配置文件路径
pub fn generate_opencode_configs(
    dir: &Path,
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
) -> std::io::Result<HashMap<String, PathBuf>> {
    let opencode_dir = dir.join("opencode");
    fs::create_dir_all(&opencode_dir)?;

    // 用 HashMap 索引模板，避免每次线性查找
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

        // 查找当前 instance 对应的模型模板
        let model_template = template.models.iter().find(|m| m.id == instance.model_id);

        // 确定使用的 opencode model id：优先用 instance 上设置的，否则 fallback 到模板 model 的映射
        let opencode_model_id = if instance.opencode_model_id.is_empty() {
            model_template
                .map(|m| m.opencode_model_id.clone())
                .unwrap_or_default()
        } else {
            instance.opencode_model_id.clone()
        };

        if template.opencode_provider_id.is_empty() || opencode_model_id.is_empty() {
            continue;
        }

        // json! 宏会 consume 其参数，因此需要提前 clone
        let provider_id = template.opencode_provider_id.clone();
        let npm = template.opencode_npm.clone();
        let name = template.name.clone();
        let base_url = template.opencode_base_url.clone();
        let env_var = template.opencode_env_var.clone();
        let model_name = model_template
            .map(|m| m.name.clone())
            .unwrap_or_else(|| opencode_model_id.clone());

        let config = json!({
            "$schema": "https://opencode.ai/config.json",
            "model": format!("{}/{}", provider_id, opencode_model_id),
            "provider": {
                provider_id: {
                    "npm": npm,
                    "name": name,
                    "options": {
                        "baseURL": base_url,
                        "apiKey": format!("{{env:{}}}", env_var)
                    },
                    "models": {
                        opencode_model_id: {
                            "name": model_name
                        }
                    }
                }
            }
        });

        let file_name = format!("{}.json", instance.alias);
        let file_path = opencode_dir.join(&file_name);
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&file_path, json)?;
        // 设置权限为 600，避免其他用户读取
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600))?;
        result.insert(instance.alias.clone(), file_path);
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

        let suffix = instance.alias.strip_prefix("cl-").unwrap_or(&instance.alias);
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
