use crate::error::Result;
use crate::{saga, session, step};
use std::path::Path;

pub fn run(saga_path: &Path, step_number: Option<u32>) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);

    if let Some(num) = step_number {
        return show_step_transcript(&saga_dir, num);
    }

    let sessions_dir = saga_dir.join("sessions");
    if show_session_transcript(&sessions_dir)? {
        return Ok(());
    }

    show_legacy_transcripts(&saga_dir, &config.name)
}

fn show_step_transcript(saga_dir: &Path, num: u32) -> Result<()> {
    let step_dir = step::find_step_dir(saga_dir, num)?;
    let step_config = step::load_step(&step_dir)?;

    if let Some(ref transcript_file) = step_config.transcript_file {
        let path = saga_dir.join(transcript_file);
        if path.is_file() {
            println!(
                "=== Transcript for step {:03}-{} ===",
                step_config.number, step_config.slug
            );
            println!("{}", std::fs::read_to_string(&path)?);
            return Ok(());
        }
    }
    println!("No transcript found for step {:03}.", num);
    Ok(())
}

/// Show the latest session JSONL transcript. Returns true if one was found.
fn show_session_transcript(sessions_dir: &Path) -> Result<bool> {
    if !sessions_dir.is_dir() {
        return Ok(false);
    }

    let mut snapshots: Vec<_> = std::fs::read_dir(sessions_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .collect();

    snapshots.sort_by_key(|e| {
        e.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    let Some(latest) = snapshots.last() else {
        return Ok(false);
    };

    println!(
        "=== Session Transcript ({}) ===\n",
        latest.file_name().to_string_lossy()
    );
    let conversation = session::extract_conversation(&latest.path())?;
    if conversation.is_empty() {
        println!("(no user/assistant messages found)");
    } else {
        println!("{conversation}");
    }

    if snapshots.len() > 1 {
        println!(
            "--- {} more session snapshot(s) available ---",
            snapshots.len() - 1
        );
        for s in &snapshots[..snapshots.len() - 1] {
            println!("  {}", s.file_name().to_string_lossy());
        }
    }
    Ok(true)
}

fn show_legacy_transcripts(saga_dir: &Path, saga_name: &str) -> Result<()> {
    let mut transcripts: Vec<_> = std::fs::read_dir(saga_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.ends_with("-transcript.txt"))
        })
        .collect();

    transcripts.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    if transcripts.is_empty() {
        println!("No transcripts found for saga '{saga_name}'.");
        return Ok(());
    }

    let latest = &transcripts[0];
    println!(
        "=== Latest Transcript ({}) ===",
        latest.file_name().to_string_lossy()
    );
    println!("{}", std::fs::read_to_string(latest.path())?);

    if transcripts.len() > 1 {
        println!(
            "\n--- {} more transcript(s) available ---",
            transcripts.len() - 1
        );
        for t in &transcripts[1..] {
            println!("  {}", t.file_name().to_string_lossy());
        }
    }
    Ok(())
}
