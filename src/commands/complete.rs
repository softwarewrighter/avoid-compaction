use crate::error::{Error, Result};
use crate::{SagaStatus, StepStatus, read_input, saga, session, step};
use std::path::Path;

pub struct CompleteArgs<'a> {
    pub transcript: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub next_prompt: Option<&'a str>,
    pub next_slug: Option<&'a str>,
    pub next_context: Vec<String>,
    pub done: bool,
}

pub fn run(saga_path: &Path, args: &CompleteArgs<'_>) -> Result<()> {
    let mut config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);

    // Snapshot the active session JSONL automatically
    let cwd = saga_path
        .canonicalize()
        .unwrap_or_else(|_| saga_path.to_path_buf());
    match session::snapshot_session(&saga_dir, &cwd) {
        Ok((path, new_lines)) => {
            println!(
                "Session snapshot: {} ({new_lines} new lines)",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        Err(e) => {
            eprintln!("Warning: could not snapshot session: {e}");
        }
    }

    // Handle the "first step" case: current_step == 0, no step to complete
    if config.current_step == 0 {
        // Save transcript if provided (legacy support)
        if let Some(transcript_val) = args.transcript {
            let content = read_input(transcript_val)?;
            let path = step::save_transcript(&saga_dir, &content)?;
            println!("Transcript saved: {}", path.display());
        }

        if args.done {
            config.status = SagaStatus::Completed;
            saga::save_saga(saga_path, &config)?;
            println!("Saga '{}' marked complete.", config.name);
            return Ok(());
        }

        // Create first step
        if let (Some(slug), Some(prompt_val)) = (args.next_slug, args.next_prompt) {
            let prompt_content = read_input(prompt_val)?;
            let description = prompt_content.lines().next().unwrap_or(slug).to_string();

            config.current_step = 1;
            step::create_step(
                &saga_dir,
                1,
                slug,
                &prompt_content,
                &description,
                &args.next_context,
            )?;
            saga::save_saga(saga_path, &config)?;
            println!("Created step 001-{}.", slug);
        } else {
            return Err(Error::Other(
                "First completion requires --next-slug and --next-prompt to create the first step."
                    .to_string(),
            ));
        }

        return Ok(());
    }

    // Normal case: complete the current step
    let step_dir = step::find_step_dir(&saga_dir, config.current_step)?;
    let mut step_config = step::load_step(&step_dir)?;

    // Allow completing a pending step (auto-transition through in-progress)
    if step_config.status == StepStatus::Pending {
        step::transition_step(&mut step_config, StepStatus::InProgress)?;
    }

    // Save transcript if provided (legacy support)
    if let Some(transcript_val) = args.transcript {
        let content = read_input(transcript_val)?;
        let path = step::save_transcript(&saga_dir, &content)?;
        step_config.transcript_file = Some(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        println!("Transcript saved: {}", path.display());
    }

    // Save summary
    if let Some(summary_val) = args.summary {
        let content = read_input(summary_val)?;
        step::save_summary(&step_dir, &content)?;
        println!(
            "Summary saved for step {:03}-{}.",
            step_config.number, step_config.slug
        );
    }

    // Mark completed
    step::transition_step(&mut step_config, StepStatus::Completed)?;
    step::save_step(&step_dir, &step_config)?;
    println!(
        "Step {:03}-{} completed.",
        step_config.number, step_config.slug
    );

    if args.done {
        config.status = SagaStatus::Completed;
        saga::save_saga(saga_path, &config)?;
        println!("Saga '{}' marked complete.", config.name);
        return Ok(());
    }

    // Create next step if specified
    if let (Some(slug), Some(prompt_val)) = (args.next_slug, args.next_prompt) {
        let prompt_content = read_input(prompt_val)?;
        let description = prompt_content.lines().next().unwrap_or(slug).to_string();
        let next_number = config.current_step + 1;

        step::create_step(
            &saga_dir,
            next_number,
            slug,
            &prompt_content,
            &description,
            &args.next_context,
        )?;

        config.current_step = next_number;
        saga::save_saga(saga_path, &config)?;
        println!("Created step {:03}-{}.", next_number, slug);
    } else {
        println!("Warning: no next step defined. Use --next-slug and --next-prompt, or --done.");
    }

    Ok(())
}
