use crate::error::Result;
use crate::{saga, session, step};
use std::path::Path;

pub fn run(saga_path: &Path, step_number: Option<u32>) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);

    // If a step number is given, show that step's transcript
    if let Some(num) = step_number {
        let step_dir = step::find_step_dir(&saga_dir, num)?;
        let step_config = step::load_step(&step_dir)?;

        // First try legacy transcript file
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
        return Ok(());
    }

    // Check for JSONL session snapshots first
    let sessions_dir = saga_dir.join("sessions");
    if sessions_dir.is_dir() {
        let mut snapshots: Vec<_> = std::fs::read_dir(&sessions_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();

        snapshots.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        if !snapshots.is_empty() {
            let latest = snapshots.last().unwrap();
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
            return Ok(());
        }
    }

    // Fall back to legacy transcript files
    let mut transcripts: Vec<_> = std::fs::read_dir(&saga_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.ends_with("-transcript.txt"))
        })
        .collect();

    transcripts.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    if transcripts.is_empty() {
        println!("No transcripts found for saga '{}'.", config.name);
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
