use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;
use cc_switch_tui::dao::Dao;
use cc_switch_tui::domain::{AppError, ModelTemplate, ProviderInstance, ProviderTemplate};
use chrono::Utc;
use std::collections::HashMap;

fn create_test_template() -> ProviderTemplate {
    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        default_env: HashMap::new(),
        models: vec![ModelTemplate {
            id: "m1".to_string(),
            name: "Model 1".to_string(),
            env_overrides: HashMap::new(),
        }],
    }
}

fn create_test_instance() -> ProviderInstance {
    ProviderInstance {
        id: "minimax-m1".to_string(),
        template_id: "minimax".to_string(),
        model_id: "m1".to_string(),
        api_key: "test-key".to_string(),
        created_at: Utc::now(),
    }
}

#[test]
fn test_create_and_get_instance() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);
    let instance = create_test_instance();

    dao.create_instance(instance.clone()).unwrap();
    let retrieved = dao.get_instance("minimax-m1").unwrap();
    assert_eq!(retrieved.id, "minimax-m1");
    assert_eq!(retrieved.api_key, "test-key");
}

#[test]
fn test_create_duplicate_instance_fails() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);
    let instance = create_test_instance();

    dao.create_instance(instance.clone()).unwrap();
    let result = dao.create_instance(instance);
    assert_eq!(result, Err(AppError::InstanceAlreadyExists("minimax-m1".to_string())));
}

#[test]
fn test_delete_instance() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);
    let instance = create_test_instance();

    dao.create_instance(instance).unwrap();
    dao.delete_instance("minimax-m1").unwrap();
    assert!(dao.get_instance("minimax-m1").is_none());
}

#[test]
fn test_delete_nonexistent_instance_fails() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);

    let result = dao.delete_instance("not-exist");
    assert_eq!(result, Err(AppError::InstanceNotFound("not-exist".to_string())));
}

#[test]
fn test_set_and_get_current_instance() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);
    let instance = create_test_instance();

    dao.create_instance(instance).unwrap();
    dao.set_current_instance("minimax-m1").unwrap();
    let current = dao.get_current_instance().unwrap();
    assert_eq!(current.id, "minimax-m1");
}

#[test]
fn test_set_current_nonexistent_instance_fails() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);

    let result = dao.set_current_instance("not-exist");
    assert_eq!(result, Err(AppError::InstanceNotFound("not-exist".to_string())));
}

#[test]
fn test_get_template() {
    let template = create_test_template();
    let dao = MemoryDaoImpl::new(vec![template]);

    let t = dao.get_template("minimax").unwrap();
    assert_eq!(t.id, "minimax");
    assert!(dao.get_template("not-exist").is_none());
}

#[test]
fn test_update_instance() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);
    let instance = create_test_instance();

    dao.create_instance(instance).unwrap();
    dao.update_instance("minimax-m1", "new-key".to_string()).unwrap();
    let updated = dao.get_instance("minimax-m1").unwrap();
    assert_eq!(updated.api_key, "new-key");
}

#[test]
fn test_update_nonexistent_instance_fails() {
    let template = create_test_template();
    let mut dao = MemoryDaoImpl::new(vec![template]);

    let result = dao.update_instance("not-exist", "key".to_string());
    assert_eq!(result, Err(AppError::InstanceNotFound("not-exist".to_string())));
}
