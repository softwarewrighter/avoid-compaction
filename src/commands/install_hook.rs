//! Install Claude Code SessionStart hook for automatic context injection.

use crate::error::Result;
use serde_json::{Map, Value};
use std::path::Path;

/// Build the hook command string that detects a saga and runs `next`.
fn hook_command() -> String {
    "if [ -f .avoid-compaction/saga.toml ]; then avoid-compaction next 2>/dev/null || cargo run --quiet -- next; fi".to_string()
}

/// Build the SessionStart hook JSON structure.
fn session_start_hook() -> Value {
    serde_json::json!([{
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": hook_command()
        }]
    }])
}

/// Install the SessionStart hook into `.claude/settings.json`.
pub fn run(project_path: &Path) -> Result<()> {
    let claude_dir = project_path.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let settings_path = claude_dir.join("settings.json");
    let mut settings = load_or_empty(&settings_path)?;

    let hooks = settings
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));

    let hooks_map = hooks
        .as_object_mut()
        .ok_or_else(|| crate::error::Error::Other("hooks is not an object".to_string()))?;

    if hooks_map.contains_key("SessionStart") {
        println!("SessionStart hook already configured in .claude/settings.json");
        println!("To update, remove the existing hook and re-run install-hook.");
        return Ok(());
    }

    hooks_map.insert("SessionStart".to_string(), session_start_hook());

    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| crate::error::Error::Other(format!("JSON serialize error: {e}")))?;
    std::fs::write(&settings_path, format!("{json}\n"))?;

    println!("Installed SessionStart hook in .claude/settings.json");
    println!("On each session start, 'avoid-compaction next' will run automatically.");
    Ok(())
}

fn load_or_empty(path: &Path) -> Result<Map<String, Value>> {
    if path.is_file() {
        let content = std::fs::read_to_string(path)?;
        let val: Value = serde_json::from_str(&content)
            .map_err(|e| crate::error::Error::Other(format!("JSON parse error: {e}")))?;
        val.as_object()
            .cloned()
            .ok_or_else(|| crate::error::Error::Other("settings.json is not an object".to_string()))
    } else {
        Ok(Map::new())
    }
}
