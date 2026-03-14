use crate::error::{Error, Result};
use crate::{SagaStatus, StepStatus, read_input, saga, session, step};
use std::path::Path;

pub struct CompleteArgs<'a> {
    pub transcript: Option<&'a str>,
    pub summary: Option<&'a str>,
    pub next_prompt: Option<&'a str>,
    pub next_slug: Option<&'a str>,
    pub next_context: Vec<String>,
    pub planned: Vec<String>,
    pub done: bool,
}

pub fn run(saga_path: &Path, args: &CompleteArgs<'_>) -> Result<()> {
    let mut config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);

    snapshot_session_quietly(&saga_dir, saga_path);

    if config.current_step == 0 {
        return handle_step0_completion(saga_path, &saga_dir, &mut config, args);
    }

    complete_current_step(saga_path, &saga_dir, &mut config, args)
}

fn snapshot_session_quietly(saga_dir: &Path, saga_path: &Path) {
    let cwd = saga_path
        .canonicalize()
        .unwrap_or_else(|_| saga_path.to_path_buf());
    match session::snapshot_session(saga_dir, &cwd) {
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
}

fn handle_step0_completion(
    saga_path: &Path,
    saga_dir: &Path,
    config: &mut crate::SagaConfig,
    args: &CompleteArgs<'_>,
) -> Result<()> {
    if let Some(transcript_val) = args.transcript {
        let content = read_input(transcript_val)?;
        let path = step::save_transcript(saga_dir, &content)?;
        println!("Transcript saved: {}", path.display());
    }

    if let Some(summary_val) = args.summary {
        let content = read_input(summary_val)?;
        let summary_path = saga_dir.join("step0-summary.md");
        std::fs::write(&summary_path, &content)?;
        println!("Step 0 summary saved.");
    }

    if args.done {
        config.status = SagaStatus::Completed;
        saga::save_saga(saga_path, config)?;
        println!("Saga '{}' marked complete.", config.name);
        return Ok(());
    }

    if let (Some(slug), Some(prompt_val)) = (args.next_slug, args.next_prompt) {
        let prompt_content = read_input(prompt_val)?;
        let description = prompt_content.lines().next().unwrap_or(slug).to_string();
        config.current_step = 1;
        step::create_step(
            saga_dir,
            1,
            slug,
            &prompt_content,
            &description,
            &args.next_context,
        )?;
        saga::save_saga(saga_path, config)?;
        println!("Created step 001-{slug}.");
    } else {
        return Err(Error::Other(
            "First completion requires --next-slug and --next-prompt to create the first step."
                .to_string(),
        ));
    }

    save_planned_steps(saga_dir, &args.planned, args.next_slug)?;
    Ok(())
}

fn complete_current_step(
    saga_path: &Path,
    saga_dir: &Path,
    config: &mut crate::SagaConfig,
    args: &CompleteArgs<'_>,
) -> Result<()> {
    let step_dir = step::find_step_dir(saga_dir, config.current_step)?;
    let mut step_config = step::load_step(&step_dir)?;

    if step_config.status == StepStatus::Pending {
        step::transition_step(&mut step_config, StepStatus::InProgress)?;
    }

    save_step_artifacts(saga_dir, &step_dir, &mut step_config, args)?;

    step::transition_step(&mut step_config, StepStatus::Completed)?;
    step::save_step(&step_dir, &step_config)?;
    println!(
        "Step {:03}-{} completed.",
        step_config.number, step_config.slug
    );

    if args.done {
        config.status = SagaStatus::Completed;
        saga::save_saga(saga_path, config)?;
        println!("Saga '{}' marked complete.", config.name);
        return Ok(());
    }

    create_next_step(saga_path, saga_dir, config, args)?;
    save_planned_steps(saga_dir, &args.planned, args.next_slug)?;
    print_restart_message(config, saga_path)?;
    Ok(())
}

fn save_step_artifacts(
    saga_dir: &Path,
    step_dir: &Path,
    step_config: &mut crate::StepConfig,
    args: &CompleteArgs<'_>,
) -> Result<()> {
    if let Some(transcript_val) = args.transcript {
        let content = read_input(transcript_val)?;
        let path = step::save_transcript(saga_dir, &content)?;
        step_config.transcript_file = Some(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        println!("Transcript saved: {}", path.display());
    }

    if let Some(summary_val) = args.summary {
        let content = read_input(summary_val)?;
        step::save_summary(step_dir, &content)?;
        println!(
            "Summary saved for step {:03}-{}.",
            step_config.number, step_config.slug
        );
    }
    Ok(())
}

fn create_next_step(
    saga_path: &Path,
    saga_dir: &Path,
    config: &mut crate::SagaConfig,
    args: &CompleteArgs<'_>,
) -> Result<()> {
    if let (Some(slug), Some(prompt_val)) = (args.next_slug, args.next_prompt) {
        let prompt_content = read_input(prompt_val)?;
        let description = prompt_content.lines().next().unwrap_or(slug).to_string();
        let next_number = config.current_step + 1;
        step::create_step(
            saga_dir,
            next_number,
            slug,
            &prompt_content,
            &description,
            &args.next_context,
        )?;
        config.current_step = next_number;
        saga::save_saga(saga_path, config)?;
        println!("Created step {:03}-{slug}.", next_number);
    } else {
        println!("Warning: no next step defined. Use --next-slug and --next-prompt, or --done.");
    }
    Ok(())
}

fn save_planned_steps(saga_dir: &Path, planned: &[String], next_slug: Option<&str>) -> Result<()> {
    if planned.is_empty() {
        return Ok(());
    }
    let filtered: Vec<&String> = planned
        .iter()
        .filter(|p| {
            // Remove the step that was just created as the next step
            if let Some(slug) = next_slug {
                !p.starts_with(&format!("{slug}:"))
            } else {
                true
            }
        })
        .collect();

    if filtered.is_empty() {
        return Ok(());
    }

    let content: String = filtered.iter().map(|p| format!("{p}\n")).collect();
    let path = saga_dir.join("planned-steps.md");
    std::fs::write(&path, content)?;
    println!("Planned steps saved ({} remaining).", filtered.len());
    Ok(())
}

fn print_restart_message(config: &crate::SagaConfig, saga_path: &Path) -> Result<()> {
    let saga_dir = saga::saga_dir(saga_path);
    let steps = step::list_steps(&saga_dir)?;

    let completed_count = steps
        .iter()
        .filter(|(_, s)| s.status == crate::StepStatus::Completed)
        .count();

    println!();
    println!("=== Status ===");

    for (dir, s) in &steps {
        if s.status == crate::StepStatus::Completed {
            let summary = read_first_line_summary(dir);
            println!("  [x] {:03}-{}: {}", s.number, s.slug, summary);
        } else {
            println!("  [ ] {:03}-{}: {}", s.number, s.slug, s.description);
        }
    }
    print_planned_steps(&saga_dir);

    println!();
    if config.status == crate::SagaStatus::Completed {
        println!(
            "Saga '{}' is finished. All {completed_count} step(s) complete.",
            config.name
        );
    } else {
        let next_step = config.current_step;
        println!(
            "{completed_count} step(s) complete. Next up: step {:03}.",
            next_step
        );
        println!("You may Ctrl-C and restart. Give the new agent: avoid-compaction next");
    }

    Ok(())
}

fn print_planned_steps(saga_dir: &Path) {
    let planned_path = saga_dir.join("planned-steps.md");
    if planned_path.is_file()
        && let Ok(content) = std::fs::read_to_string(&planned_path)
    {
        for line in content.lines() {
            if !line.trim().is_empty() {
                println!("  ( ) {line}  (planned)");
            }
        }
    }
}

fn read_first_line_summary(step_dir: &Path) -> String {
    let summary_path = step_dir.join("summary.md");
    if summary_path.is_file()
        && let Ok(content) = std::fs::read_to_string(&summary_path)
    {
        return content.lines().next().unwrap_or("(done)").to_string();
    }
    "(no summary)".to_string()
}
