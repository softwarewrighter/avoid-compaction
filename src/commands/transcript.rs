use crate::error::Result;
use crate::saga;
use crate::step;
use std::path::Path;

pub fn run(saga_path: &Path, step_number: Option<u32>) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);

    // If a step number is given, show that step's transcript
    if let Some(num) = step_number {
        let step_dir = step::find_step_dir(&saga_dir, num)?;
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
        return Ok(());
    }

    // Otherwise, list all transcript files (most recent first)
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

    // Show most recent transcript
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
