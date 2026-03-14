//! Reads and snapshots Claude Code session JSONL files from ~/.claude/projects/.
//!
//! Claude Code stores session history as append-only JSONL files at:
//!   ~/.claude/projects/{cwd-with-slashes-as-dashes}/{sessionId}.jsonl
//!
//! These files survive compaction (which only affects in-memory context).

use crate::error::{Error, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Derive the Claude projects directory name from a working directory path.
/// Replaces `/` with `-` so `/Users/mike/proj` becomes `-Users-mike-proj`.
pub fn projects_dir_name(cwd: &Path) -> String {
    let abs = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    abs.to_string_lossy().replace('/', "-")
}

/// Return the full path to the Claude projects directory for a given cwd.
pub fn claude_projects_dir(cwd: &Path) -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| Error::Other("Cannot determine home directory".into()))?;
    let dir_name = projects_dir_name(cwd);
    let dir = home.join(".claude").join("projects").join(&dir_name);
    Ok(dir)
}

/// Find all session JSONL files in a Claude projects directory.
/// Returns paths sorted by modification time (most recent last).
pub fn find_session_files(projects_dir: &Path) -> Result<Vec<PathBuf>> {
    if !projects_dir.is_dir() {
        return Ok(vec![]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(projects_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .map(|e| e.path())
        .collect();

    files.sort_by_key(|p| {
        p.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    Ok(files)
}

/// A parsed JSONL record from a Claude Code session.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub record_type: String,
    pub timestamp: Option<String>,
    pub content: String,
}

/// Parse a single JSONL line into a SessionRecord.
fn parse_record(line: &str) -> Option<SessionRecord> {
    let v: Value = serde_json::from_str(line).ok()?;
    let record_type = v.get("type")?.as_str()?.to_string();
    let timestamp = v
        .get("timestamp")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    Some(SessionRecord {
        record_type,
        timestamp,
        content: line.to_string(),
    })
}

/// Extract human-readable conversation text from a JSONL file.
/// Returns user messages, assistant text, and tool summaries.
pub fn extract_conversation(jsonl_path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(jsonl_path)?;
    let mut output = String::new();

    for line in content.lines() {
        let Some(record) = parse_record(line) else {
            continue;
        };
        let v: Value = match serde_json::from_str(&record.content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match record.record_type.as_str() {
            "user" => {
                if let Some(msg) = v.get("message") {
                    let text = extract_message_text(msg);
                    if !text.is_empty() {
                        let ts = record.timestamp.as_deref().unwrap_or("?");
                        output.push_str(&format!("[{ts}] USER:\n{text}\n\n"));
                    }
                }
            }
            "assistant" => {
                if let Some(msg) = v.get("message") {
                    let text = extract_assistant_text(msg);
                    if !text.is_empty() {
                        let ts = record.timestamp.as_deref().unwrap_or("?");
                        output.push_str(&format!("[{ts}] ASSISTANT:\n{text}\n\n"));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(output)
}

/// Extract text content from a user message.
fn extract_message_text(msg: &Value) -> String {
    if let Some(content) = msg.get("content") {
        if let Some(s) = content.as_str() {
            return s.trim().to_string();
        }
        if let Some(arr) = content.as_array() {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                }
            }
            return parts.join("\n");
        }
    }
    String::new()
}

/// Extract text blocks from an assistant message (skip tool_use blocks).
fn extract_assistant_text(msg: &Value) -> String {
    if let Some(arr) = msg.get("content").and_then(|c| c.as_array()) {
        let mut parts = Vec::new();
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("text")
                && let Some(text) = item.get("text").and_then(|t| t.as_str())
            {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
            if item.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                && let Some(name) = item.get("name").and_then(|n| n.as_str())
            {
                parts.push(format!("[tool: {name}]"));
            }
        }
        return parts.join("\n");
    }
    String::new()
}

/// Snapshot the latest session JSONL into the saga's sessions directory.
/// Returns the path to the snapshot file and the number of new lines added.
pub fn snapshot_session(saga_dir: &Path, cwd: &Path) -> Result<(PathBuf, usize)> {
    let projects_dir = claude_projects_dir(cwd)?;
    let session_files = find_session_files(&projects_dir)?;

    let latest = session_files.last().ok_or_else(|| {
        Error::Other(format!(
            "No session JSONL files found in {}",
            projects_dir.display()
        ))
    })?;

    let sessions_dir = saga_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;

    let session_name = latest
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let snapshot_path = sessions_dir.join(format!("{session_name}.jsonl"));

    // Read the current source file
    let source_content = std::fs::read_to_string(latest)?;
    let source_lines: Vec<&str> = source_content.lines().collect();

    // Read existing snapshot to find how many lines we already have
    let existing_lines = if snapshot_path.is_file() {
        let existing = std::fs::read_to_string(&snapshot_path)?;
        existing.lines().count()
    } else {
        0
    };

    let new_lines = source_lines.len().saturating_sub(existing_lines);

    if new_lines > 0 {
        // Append only the new lines
        let new_content: String = source_lines[existing_lines..]
            .iter()
            .map(|l| format!("{l}\n"))
            .collect();

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&snapshot_path)?;
        file.write_all(new_content.as_bytes())?;
    }

    Ok((snapshot_path, new_lines))
}

/// Diff two snapshots of the same session, returning lines present in `older`
/// but missing from `newer` (i.e., content lost to compaction).
/// Since JSONL is append-only, this compares by line count: if newer has fewer
/// lines than older, something was truncated. In practice, newer should always
/// have >= lines, so this returns the delta of new lines only.
pub fn diff_snapshots(older: &Path, newer: &Path) -> Result<DiffResult> {
    let old_content = std::fs::read_to_string(older)?;
    let new_content = std::fs::read_to_string(newer)?;
    let old_count = old_content.lines().count();
    let new_count = new_content.lines().count();

    Ok(DiffResult {
        old_lines: old_count,
        new_lines: new_count,
        added: new_count.saturating_sub(old_count),
    })
}

#[derive(Debug)]
pub struct DiffResult {
    pub old_lines: usize,
    pub new_lines: usize,
    pub added: usize,
}
