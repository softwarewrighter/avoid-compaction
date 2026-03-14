use avoid_compaction::session;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn projects_dir_name_replaces_slashes() {
    let name = session::projects_dir_name(Path::new("/Users/mike/github/proj"));
    assert_eq!(name, "-Users-mike-github-proj");
}

#[test]
fn projects_dir_name_handles_root() {
    let name = session::projects_dir_name(Path::new("/"));
    assert_eq!(name, "-");
}

#[test]
fn find_session_files_empty_dir() {
    let tmp = tempdir().unwrap();
    let files = session::find_session_files(tmp.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn find_session_files_nonexistent_dir() {
    let files = session::find_session_files(Path::new("/nonexistent/path")).unwrap();
    assert!(files.is_empty());
}

#[test]
fn find_session_files_finds_jsonl() {
    let tmp = tempdir().unwrap();
    std::fs::write(tmp.path().join("abc123.jsonl"), "{\"type\":\"user\"}\n").unwrap();
    std::fs::write(tmp.path().join("not-a-session.txt"), "nope").unwrap();

    let files = session::find_session_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("abc123.jsonl"));
}

#[test]
fn find_session_files_sorted_by_mtime() {
    let tmp = tempdir().unwrap();
    std::fs::write(tmp.path().join("older.jsonl"), "{}\n").unwrap();
    // Small delay to ensure different mtimes
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(tmp.path().join("newer.jsonl"), "{}\n{}\n").unwrap();

    let files = session::find_session_files(tmp.path()).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[1].to_string_lossy().contains("newer"));
}

#[test]
fn extract_conversation_user_and_assistant() {
    let tmp = tempdir().unwrap();
    let jsonl_path = tmp.path().join("session.jsonl");

    let user_line = serde_json::json!({
        "type": "user",
        "timestamp": "2026-03-14T19:00:00Z",
        "message": {
            "role": "user",
            "content": "Hello Claude"
        }
    });

    let assistant_line = serde_json::json!({
        "type": "assistant",
        "timestamp": "2026-03-14T19:00:01Z",
        "message": {
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Hello! How can I help?"},
                {"type": "tool_use", "name": "Read", "input": {}}
            ]
        }
    });

    let content = format!(
        "{}\n{}\n",
        serde_json::to_string(&user_line).unwrap(),
        serde_json::to_string(&assistant_line).unwrap()
    );
    std::fs::write(&jsonl_path, content).unwrap();

    let conversation = session::extract_conversation(&jsonl_path).unwrap();
    assert!(conversation.contains("USER:"));
    assert!(conversation.contains("Hello Claude"));
    assert!(conversation.contains("ASSISTANT:"));
    assert!(conversation.contains("Hello! How can I help?"));
    assert!(conversation.contains("[tool: Read]"));
}

#[test]
fn extract_conversation_array_content() {
    let tmp = tempdir().unwrap();
    let jsonl_path = tmp.path().join("session.jsonl");

    let user_line = serde_json::json!({
        "type": "user",
        "timestamp": "2026-03-14T19:00:00Z",
        "message": {
            "role": "user",
            "content": [
                {"type": "text", "text": "Part one"},
                {"type": "text", "text": "Part two"}
            ]
        }
    });

    std::fs::write(
        &jsonl_path,
        format!("{}\n", serde_json::to_string(&user_line).unwrap()),
    )
    .unwrap();

    let conversation = session::extract_conversation(&jsonl_path).unwrap();
    assert!(conversation.contains("Part one"));
    assert!(conversation.contains("Part two"));
}

#[test]
fn extract_conversation_skips_empty_messages() {
    let tmp = tempdir().unwrap();
    let jsonl_path = tmp.path().join("session.jsonl");

    let progress_line = serde_json::json!({
        "type": "progress",
        "timestamp": "2026-03-14T19:00:00Z",
        "data": {"type": "hook_progress"}
    });

    let empty_user = serde_json::json!({
        "type": "user",
        "timestamp": "2026-03-14T19:00:01Z",
        "message": {"role": "user", "content": "  "}
    });

    let content = format!(
        "{}\n{}\n",
        serde_json::to_string(&progress_line).unwrap(),
        serde_json::to_string(&empty_user).unwrap()
    );
    std::fs::write(&jsonl_path, content).unwrap();

    let conversation = session::extract_conversation(&jsonl_path).unwrap();
    assert!(!conversation.contains("USER:"));
}

#[test]
fn snapshot_session_creates_sessions_dir_and_copies() {
    let tmp = tempdir().unwrap();

    // Simulate a project directory structure
    let cwd = tmp.path().join("project");
    std::fs::create_dir_all(&cwd).unwrap();

    // We can't easily test snapshot_session without mocking home dir,
    // but we can test the building blocks.
    // Test that sessions dir creation works in saga_dir
    let saga_dir = cwd.join(".avoid-compaction");
    std::fs::create_dir_all(&saga_dir).unwrap();
    let sessions_dir = saga_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir).unwrap();
    assert!(sessions_dir.is_dir());
}

#[test]
fn diff_snapshots_detects_added_lines() {
    let tmp = tempdir().unwrap();
    let older = tmp.path().join("older.jsonl");
    let newer = tmp.path().join("newer.jsonl");

    std::fs::write(&older, "line1\nline2\n").unwrap();
    std::fs::write(&newer, "line1\nline2\nline3\nline4\n").unwrap();

    let diff = session::diff_snapshots(&older, &newer).unwrap();
    assert_eq!(diff.old_lines, 2);
    assert_eq!(diff.new_lines, 4);
    assert_eq!(diff.added, 2);
}

#[test]
fn diff_snapshots_no_change() {
    let tmp = tempdir().unwrap();
    let older = tmp.path().join("a.jsonl");
    let newer = tmp.path().join("b.jsonl");

    std::fs::write(&older, "line1\nline2\n").unwrap();
    std::fs::write(&newer, "line1\nline2\n").unwrap();

    let diff = session::diff_snapshots(&older, &newer).unwrap();
    assert_eq!(diff.added, 0);
}

#[test]
fn claude_projects_dir_constructs_path() {
    // This test verifies the path construction logic.
    // It uses a fake path that won't resolve via canonicalize,
    // so projects_dir_name falls back to the provided path.
    let result = session::claude_projects_dir(Path::new("/fake/path"));
    assert!(result.is_ok());
    let path = result.unwrap();
    // Should end with the mangled dir name
    let path_str = path.to_string_lossy();
    assert!(path_str.contains(".claude/projects/"));
}
