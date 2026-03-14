use avoid_compaction::StepStatus;
use avoid_compaction::error::Error;
use avoid_compaction::saga;
use avoid_compaction::step;
use tempfile::tempdir;

fn setup_saga() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();
    let saga_dir = saga::saga_dir(tmp.path());
    (tmp, saga_dir)
}

#[test]
fn create_step_creates_directory_and_files() {
    let (_tmp, saga_dir) = setup_saga();

    let dir = step::create_step(
        &saga_dir,
        1,
        "setup",
        "Do the setup work",
        "Set up the project",
        &["Cargo.toml".to_string(), "src/main.rs".to_string()],
    )
    .unwrap();

    assert!(dir.join("step.toml").is_file());
    assert!(dir.join("prompt.md").is_file());

    let prompt = std::fs::read_to_string(dir.join("prompt.md")).unwrap();
    assert_eq!(prompt, "Do the setup work");
}

#[test]
fn create_step_sets_correct_defaults() {
    let (_tmp, saga_dir) = setup_saga();

    let dir = step::create_step(&saga_dir, 1, "init", "prompt", "desc", &[]).unwrap();
    let config = step::load_step(&dir).unwrap();

    assert_eq!(config.number, 1);
    assert_eq!(config.slug, "init");
    assert_eq!(config.status, StepStatus::Pending);
    assert_eq!(config.description, "desc");
    assert!(config.context_files.is_empty());
    assert!(config.completed_at.is_none());
    assert!(config.transcript_file.is_none());
}

#[test]
fn find_step_dir_locates_by_number() {
    let (_tmp, saga_dir) = setup_saga();

    step::create_step(&saga_dir, 1, "first", "p", "d", &[]).unwrap();
    step::create_step(&saga_dir, 2, "second", "p", "d", &[]).unwrap();

    let found = step::find_step_dir(&saga_dir, 2).unwrap();
    assert!(found.ends_with("002-second"));
}

#[test]
fn find_step_dir_fails_for_missing_number() {
    let (_tmp, saga_dir) = setup_saga();
    step::create_step(&saga_dir, 1, "only", "p", "d", &[]).unwrap();

    let result = step::find_step_dir(&saga_dir, 99);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::NoCurrentStep));
}

#[test]
fn transition_pending_to_in_progress() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(&saga_dir, 1, "t", "p", "d", &[]).unwrap();
    let mut config = step::load_step(&dir).unwrap();

    assert_eq!(config.status, StepStatus::Pending);
    step::transition_step(&mut config, StepStatus::InProgress).unwrap();
    assert_eq!(config.status, StepStatus::InProgress);
    assert!(config.completed_at.is_none());
}

#[test]
fn transition_in_progress_to_completed_sets_timestamp() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(&saga_dir, 1, "t", "p", "d", &[]).unwrap();
    let mut config = step::load_step(&dir).unwrap();

    step::transition_step(&mut config, StepStatus::InProgress).unwrap();
    step::transition_step(&mut config, StepStatus::Completed).unwrap();

    assert_eq!(config.status, StepStatus::Completed);
    assert!(config.completed_at.is_some());
}

#[test]
fn transition_in_progress_to_blocked() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(&saga_dir, 1, "t", "p", "d", &[]).unwrap();
    let mut config = step::load_step(&dir).unwrap();

    step::transition_step(&mut config, StepStatus::InProgress).unwrap();
    step::transition_step(&mut config, StepStatus::Blocked).unwrap();
    assert_eq!(config.status, StepStatus::Blocked);
}

#[test]
fn transition_pending_to_completed_fails() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(&saga_dir, 1, "t", "p", "d", &[]).unwrap();
    let mut config = step::load_step(&dir).unwrap();

    let result = step::transition_step(&mut config, StepStatus::Completed);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        Error::InvalidStepTransition { .. }
    ));
}

#[test]
fn transition_completed_to_anything_fails() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(&saga_dir, 1, "t", "p", "d", &[]).unwrap();
    let mut config = step::load_step(&dir).unwrap();

    step::transition_step(&mut config, StepStatus::InProgress).unwrap();
    step::transition_step(&mut config, StepStatus::Completed).unwrap();

    let result = step::transition_step(&mut config, StepStatus::InProgress);
    assert!(result.is_err());
}

#[test]
fn list_steps_returns_sorted() {
    let (_tmp, saga_dir) = setup_saga();

    step::create_step(&saga_dir, 3, "third", "p", "d", &[]).unwrap();
    step::create_step(&saga_dir, 1, "first", "p", "d", &[]).unwrap();
    step::create_step(&saga_dir, 2, "second", "p", "d", &[]).unwrap();

    let steps = step::list_steps(&saga_dir).unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].1.number, 1);
    assert_eq!(steps[1].1.number, 2);
    assert_eq!(steps[2].1.number, 3);
}

#[test]
fn list_steps_empty_saga() {
    let (_tmp, saga_dir) = setup_saga();
    let steps = step::list_steps(&saga_dir).unwrap();
    assert!(steps.is_empty());
}

#[test]
fn save_transcript_creates_timestamped_file() {
    let (_tmp, saga_dir) = setup_saga();

    let path = step::save_transcript(&saga_dir, "User asked X. Claude did Y.").unwrap();

    assert!(path.is_file());
    let filename = path.file_name().unwrap().to_str().unwrap();
    assert!(filename.ends_with("-transcript.txt"));

    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "User asked X. Claude did Y.");
}

#[test]
fn save_summary_writes_to_step_dir() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(&saga_dir, 1, "t", "p", "d", &[]).unwrap();

    step::save_summary(&dir, "Completed the task successfully.").unwrap();

    let content = std::fs::read_to_string(dir.join("summary.md")).unwrap();
    assert_eq!(content, "Completed the task successfully.");
}

#[test]
fn save_and_load_step_roundtrips() {
    let (_tmp, saga_dir) = setup_saga();
    let dir = step::create_step(
        &saga_dir,
        1,
        "rt",
        "prompt text",
        "description",
        &["a.rs".to_string(), "b.rs".to_string()],
    )
    .unwrap();

    let mut config = step::load_step(&dir).unwrap();
    config.transcript_file = Some("20260101T000000-transcript.txt".to_string());
    step::save_step(&dir, &config).unwrap();

    let reloaded = step::load_step(&dir).unwrap();
    assert_eq!(
        reloaded.transcript_file.as_deref(),
        Some("20260101T000000-transcript.txt")
    );
    assert_eq!(reloaded.context_files, vec!["a.rs", "b.rs"]);
}
