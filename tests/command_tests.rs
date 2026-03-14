use avoid_compaction::commands::complete::CompleteArgs;
use avoid_compaction::commands::{begin, complete, init, install_hook, next, status};
use avoid_compaction::saga;
use avoid_compaction::step;
use avoid_compaction::{SagaStatus, StepStatus};
use tempfile::tempdir;

#[test]
fn init_command_creates_saga() {
    let tmp = tempdir().unwrap();
    init::run(tmp.path(), "my-feature", "Build a thing").unwrap();

    assert!(saga::saga_exists(tmp.path()));
    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.name, "my-feature");
}

#[test]
fn next_returns_2_when_no_saga() {
    let tmp = tempdir().unwrap();
    let code = next::run(tmp.path()).unwrap();
    assert_eq!(code, 2);
}

#[test]
fn next_returns_0_for_first_step() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan content").unwrap();

    let code = next::run(tmp.path()).unwrap();
    assert_eq!(code, 0);
}

#[test]
fn next_returns_1_when_saga_complete() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let mut config = saga::load_saga(tmp.path()).unwrap();
    config.status = SagaStatus::Completed;
    saga::save_saga(tmp.path(), &config).unwrap();

    let code = next::run(tmp.path()).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn next_returns_0_for_pending_step() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let saga_dir = saga::saga_dir(tmp.path());
    step::create_step(&saga_dir, 1, "first", "Do the thing", "First step", &[]).unwrap();

    let mut config = saga::load_saga(tmp.path()).unwrap();
    config.current_step = 1;
    saga::save_saga(tmp.path(), &config).unwrap();

    let code = next::run(tmp.path()).unwrap();
    assert_eq!(code, 0);
}

#[test]
fn complete_first_step_creates_step_001() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let args = CompleteArgs {
        transcript: Some("User said hello"),
        summary: None,
        next_prompt: Some("Implement feature X"),
        next_slug: Some("feature-x"),
        next_context: vec!["src/lib.rs".to_string()],
        planned: vec![],
        done: false,
    };

    complete::run(tmp.path(), &args).unwrap();

    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.current_step, 1);

    let saga_dir = saga::saga_dir(tmp.path());
    let step_dir = step::find_step_dir(&saga_dir, 1).unwrap();
    let step_config = step::load_step(&step_dir).unwrap();
    assert_eq!(step_config.slug, "feature-x");
    assert_eq!(step_config.status, StepStatus::Pending);
    assert_eq!(step_config.context_files, vec!["src/lib.rs"]);
}

#[test]
fn complete_first_step_fails_without_next_info() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let args = CompleteArgs {
        transcript: Some("stuff happened"),
        summary: None,
        next_prompt: None,
        next_slug: None,
        next_context: vec![],
        planned: vec![],
        done: false,
    };

    let result = complete::run(tmp.path(), &args);
    assert!(result.is_err());
}

#[test]
fn complete_first_step_with_done_marks_saga_complete() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let args = CompleteArgs {
        transcript: Some("nothing to do"),
        summary: None,
        next_prompt: None,
        next_slug: None,
        next_context: vec![],
        planned: vec![],
        done: true,
    };

    complete::run(tmp.path(), &args).unwrap();

    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.status, SagaStatus::Completed);
}

#[test]
fn begin_transitions_step_to_in_progress() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let saga_dir = saga::saga_dir(tmp.path());
    step::create_step(&saga_dir, 1, "first", "prompt", "desc", &[]).unwrap();

    let mut config = saga::load_saga(tmp.path()).unwrap();
    config.current_step = 1;
    saga::save_saga(tmp.path(), &config).unwrap();

    begin::run(tmp.path()).unwrap();

    let step_dir = step::find_step_dir(&saga_dir, 1).unwrap();
    let step_config = step::load_step(&step_dir).unwrap();
    assert_eq!(step_config.status, StepStatus::InProgress);
}

#[test]
fn full_workflow_init_through_done() {
    let tmp = tempdir().unwrap();

    // Init
    init::run(tmp.path(), "workflow-test", "3-step plan").unwrap();

    // First complete: create step 1
    let args = CompleteArgs {
        transcript: Some("Created plan"),
        summary: None,
        next_prompt: Some("Build the skeleton"),
        next_slug: Some("skeleton"),
        next_context: vec!["Cargo.toml".to_string()],
        planned: vec![],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    // Verify step 1 exists
    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.current_step, 1);

    // Begin step 1
    begin::run(tmp.path()).unwrap();

    // Complete step 1, create step 2
    let args = CompleteArgs {
        transcript: Some("Built skeleton"),
        summary: Some("Project skeleton created"),
        next_prompt: Some("Add core logic"),
        next_slug: Some("core-logic"),
        next_context: vec!["src/lib.rs".to_string()],
        planned: vec![],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.current_step, 2);
    assert_eq!(config.status, SagaStatus::Active);

    // Begin step 2
    begin::run(tmp.path()).unwrap();

    // Complete step 2 with --done
    let args = CompleteArgs {
        transcript: Some("Added logic"),
        summary: Some("Core logic implemented"),
        next_prompt: None,
        next_slug: None,
        next_context: vec![],
        planned: vec![],
        done: true,
    };
    complete::run(tmp.path(), &args).unwrap();

    let config = saga::load_saga(tmp.path()).unwrap();
    assert_eq!(config.status, SagaStatus::Completed);

    // Verify history
    let saga_dir = saga::saga_dir(tmp.path());
    let steps = step::list_steps(&saga_dir).unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].1.status, StepStatus::Completed);
    assert_eq!(steps[1].1.status, StepStatus::Completed);

    // Verify summaries
    let summary1 = std::fs::read_to_string(steps[0].0.join("summary.md")).unwrap();
    assert_eq!(summary1, "Project skeleton created");
    let summary2 = std::fs::read_to_string(steps[1].0.join("summary.md")).unwrap();
    assert_eq!(summary2, "Core logic implemented");
}

#[test]
fn complete_auto_transitions_pending_to_completed() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    // Create step 1 via first complete
    let args = CompleteArgs {
        transcript: None,
        summary: None,
        next_prompt: Some("Do step 1"),
        next_slug: Some("step-1"),
        next_context: vec![],
        planned: vec![],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    // Complete step 1 WITHOUT calling begin first
    let args = CompleteArgs {
        transcript: Some("did it"),
        summary: Some("done"),
        next_prompt: None,
        next_slug: None,
        next_context: vec![],
        planned: vec![],
        done: true,
    };
    complete::run(tmp.path(), &args).unwrap();

    let saga_dir = saga::saga_dir(tmp.path());
    let step_dir = step::find_step_dir(&saga_dir, 1).unwrap();
    let step_config = step::load_step(&step_dir).unwrap();
    assert_eq!(step_config.status, StepStatus::Completed);
}

#[test]
fn status_runs_without_error() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();
    status::run(tmp.path()).unwrap();
}

#[test]
fn transcript_saved_with_content() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let saga_dir = saga::saga_dir(tmp.path());
    let path = step::save_transcript(&saga_dir, "User: do X\nClaude: did X").unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("User: do X"));
    assert!(content.contains("Claude: did X"));
}

#[test]
fn read_input_from_file() {
    let tmp = tempdir().unwrap();
    let file_path = tmp.path().join("input.txt");
    std::fs::write(&file_path, "content from file").unwrap();

    let result = avoid_compaction::read_input(file_path.to_str().unwrap()).unwrap();
    assert_eq!(result, "content from file");
}

#[test]
fn read_input_literal_text() {
    let result = avoid_compaction::read_input("just some text").unwrap();
    assert_eq!(result, "just some text");
}

#[test]
fn complete_step0_saves_summary() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let args = CompleteArgs {
        transcript: None,
        summary: Some("Session 0: Built the skeleton and wrote tests"),
        next_prompt: Some("Fix the widget"),
        next_slug: Some("fix-widget"),
        next_context: vec![],
        planned: vec![],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    let summary_path = saga::saga_dir(tmp.path()).join("step0-summary.md");
    assert!(summary_path.is_file());
    let content = std::fs::read_to_string(&summary_path).unwrap();
    assert_eq!(content, "Session 0: Built the skeleton and wrote tests");
}

#[test]
fn next_shows_step0_summary_for_first_real_step() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    // Complete step 0 with summary, creating step 1
    let args = CompleteArgs {
        transcript: None,
        summary: Some("Built the initial prototype"),
        next_prompt: Some("Add error handling"),
        next_slug: Some("error-handling"),
        next_context: vec!["src/lib.rs".to_string()],
        planned: vec![],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    // next should succeed and show step 1
    let code = next::run(tmp.path()).unwrap();
    assert_eq!(code, 0);

    // Verify step0-summary.md exists (next reads it internally)
    let summary_path = saga::saga_dir(tmp.path()).join("step0-summary.md");
    assert!(summary_path.is_file());
}

#[test]
fn complete_saves_planned_steps() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let args = CompleteArgs {
        transcript: None,
        summary: Some("Did initial work"),
        next_prompt: Some("Do step 1"),
        next_slug: Some("step-1"),
        next_context: vec![],
        planned: vec![
            "step-2: Add error handling".to_string(),
            "step-3: Write tests".to_string(),
        ],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    let planned_path = saga::saga_dir(tmp.path()).join("planned-steps.md");
    assert!(planned_path.is_file());
    let content = std::fs::read_to_string(&planned_path).unwrap();
    assert!(content.contains("step-2: Add error handling"));
    assert!(content.contains("step-3: Write tests"));
}

#[test]
fn complete_removes_next_slug_from_planned() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    // Create step 1 with planned steps
    let args = CompleteArgs {
        transcript: None,
        summary: None,
        next_prompt: Some("Do step 1"),
        next_slug: Some("step-1"),
        next_context: vec![],
        planned: vec![
            "step-1: Build the thing".to_string(),
            "step-2: Test the thing".to_string(),
        ],
        done: false,
    };
    complete::run(tmp.path(), &args).unwrap();

    // step-1 was just created as the next step, so it should be removed from planned
    let planned_path = saga::saga_dir(tmp.path()).join("planned-steps.md");
    let content = std::fs::read_to_string(&planned_path).unwrap();
    assert!(!content.contains("step-1:"));
    assert!(content.contains("step-2: Test the thing"));
}

#[test]
fn init_fails_gracefully_when_saga_exists() {
    let tmp = tempdir().unwrap();
    init::run(tmp.path(), "first", "plan 1").unwrap();

    let result = init::run(tmp.path(), "second", "plan 2");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("already exists"));
}

#[test]
fn install_hook_creates_settings_file() {
    let tmp = tempdir().unwrap();
    install_hook::run(tmp.path()).unwrap();

    let settings_path = tmp.path().join(".claude/settings.json");
    assert!(settings_path.is_file());

    let content = std::fs::read_to_string(&settings_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(val["hooks"]["SessionStart"].is_array());

    let hook_cmd = val["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert!(hook_cmd.contains("avoid-compaction next"));
}

#[test]
fn install_hook_preserves_existing_settings() {
    let tmp = tempdir().unwrap();
    let claude_dir = tmp.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();

    let existing = serde_json::json!({"someKey": "someValue"});
    std::fs::write(
        claude_dir.join("settings.json"),
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    install_hook::run(tmp.path()).unwrap();

    let content = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(val["someKey"], "someValue");
    assert!(val["hooks"]["SessionStart"].is_array());
}

#[test]
fn next_warns_about_missing_context_files() {
    let tmp = tempdir().unwrap();
    saga::init_saga(tmp.path(), "test", "plan").unwrap();

    let saga_dir = saga::saga_dir(tmp.path());
    step::create_step(
        &saga_dir,
        1,
        "check-files",
        "Do work",
        "Check files",
        &["src/real.rs".to_string(), "src/missing.rs".to_string()],
    )
    .unwrap();

    // Create one of the two context files so only the other triggers a warning
    let real_path = tmp.path().join("src");
    std::fs::create_dir_all(&real_path).unwrap();
    std::fs::write(real_path.join("real.rs"), "// exists").unwrap();

    let mut config = saga::load_saga(tmp.path()).unwrap();
    config.current_step = 1;
    saga::save_saga(tmp.path(), &config).unwrap();

    // The function should succeed (exit code 0) even with missing files
    let code = next::run(tmp.path()).unwrap();
    assert_eq!(code, 0);
}

#[test]
fn install_hook_skips_if_already_configured() {
    let tmp = tempdir().unwrap();
    install_hook::run(tmp.path()).unwrap();

    let content_before = std::fs::read_to_string(tmp.path().join(".claude/settings.json")).unwrap();

    // Running again should not modify the file
    install_hook::run(tmp.path()).unwrap();

    let content_after = std::fs::read_to_string(tmp.path().join(".claude/settings.json")).unwrap();
    assert_eq!(content_before, content_after);
}
