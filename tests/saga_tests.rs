use avoid_compaction::SagaStatus;
use avoid_compaction::error::Error;
use avoid_compaction::saga;
use tempfile::tempdir;

#[test]
fn init_creates_saga_directory_and_files() {
    let tmp = tempdir().unwrap();
    let path = tmp.path();

    saga::init_saga(path, "test-saga", "This is the plan").unwrap();

    assert!(saga::saga_exists(path));

    let dir = saga::saga_dir(path);
    assert!(dir.join("saga.toml").is_file());
    assert!(dir.join("plan.md").is_file());
    assert!(dir.join("steps").is_dir());

    let plan = std::fs::read_to_string(dir.join("plan.md")).unwrap();
    assert_eq!(plan, "This is the plan");
}

#[test]
fn init_sets_correct_defaults() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "my-saga", "plan content").unwrap();

    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.name, "my-saga");
    assert_eq!(config.status, SagaStatus::Active);
    assert_eq!(config.current_step, 0);
    assert!(config.plan_file.ends_with("plan.md"));
}

#[test]
fn init_fails_if_saga_already_exists() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "first", "plan").unwrap();

    let result = saga::init_saga(tmp.path(), "second", "plan2");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Error::SagaAlreadyExists { .. }
    ));
}

#[test]
fn load_saga_fails_on_empty_dir() {
    let tmp = tempdir().unwrap();

    let result = saga::load_saga(tmp.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::SagaNotFound { .. }));
}

#[test]
fn saga_exists_returns_false_for_empty_dir() {
    let tmp = tempdir().unwrap();
    assert!(!saga::saga_exists(tmp.path()));
}

#[test]
fn save_and_load_saga_roundtrips() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "roundtrip", "plan").unwrap();

    let mut config = saga::load_saga(tmp.path()).unwrap();
    config.current_step = 5;
    config.status = SagaStatus::Completed;
    saga::save_saga(tmp.path(), &config).unwrap();

    let reloaded = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(reloaded.current_step, 5);
    assert_eq!(reloaded.status, SagaStatus::Completed);
}
