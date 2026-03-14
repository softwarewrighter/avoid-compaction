use crate::error::Result;
use crate::saga;
use crate::step;
use std::path::Path;

/// Exit codes: 0 = step available, 1 = saga complete, 2 = no saga
pub fn run(saga_path: &Path) -> Result<u8> {
    if !saga::saga_exists(saga_path) {
        println!(
            "No saga found. Initialize one with: avoid-compaction init --name <name> --plan <file>"
        );
        return Ok(2);
    }

    let config = saga::load_saga(saga_path)?;

    if config.status == crate::SagaStatus::Completed {
        println!("Saga '{}' is complete. No more steps.", config.name);
        return Ok(1);
    }

    let saga_dir = saga::saga_dir(saga_path);

    if config.current_step == 0 {
        println!("=== FIRST STEP ===");
        println!("Saga: {}", config.name);
        println!("No prior context. This is the first step.");
        println!("Plan: {}", config.plan_file);

        let plan_path = saga_path.join(&config.plan_file);
        if plan_path.is_file() {
            println!("\n--- Plan ---");
            println!("{}", std::fs::read_to_string(&plan_path)?);
        }

        println!("\nAction: Review the plan, do the first unit of work, then run:");
        println!("  avoid-compaction complete --transcript <text> --summary <text> \\");
        println!("    --next-slug <slug> --next-prompt <file> --next-context <files>");
        return Ok(0);
    }

    // Find current step
    let step_dir = step::find_step_dir(&saga_dir, config.current_step)?;
    let step_config = step::load_step(&step_dir)?;

    // Print step info
    println!(
        "=== STEP {:03}: {} ===",
        step_config.number, step_config.slug
    );
    println!("Status: {}", step_config.status);
    println!("Description: {}", step_config.description);

    // Print prompt
    let prompt_path = step_dir.join("prompt.md");
    if prompt_path.is_file() {
        println!("\n--- Prompt ---");
        println!("{}", std::fs::read_to_string(&prompt_path)?);
    }

    // Print context files (paths only)
    if !step_config.context_files.is_empty() {
        println!("\n--- Context Files ---");
        for f in &step_config.context_files {
            println!("  {}", f);
        }
    }

    // Print prior step summaries (brief)
    let steps = step::list_steps(&saga_dir)?;
    let completed: Vec<_> = steps
        .iter()
        .filter(|(_, s)| s.status == crate::StepStatus::Completed)
        .collect();

    if !completed.is_empty() {
        println!("\n--- Prior Steps ---");
        for (dir, s) in &completed {
            let summary_path = dir.join("summary.md");
            let summary = if summary_path.is_file() {
                let content = std::fs::read_to_string(&summary_path)?;
                // First 3 lines only
                content.lines().take(3).collect::<Vec<_>>().join(" ")
            } else {
                "(no summary)".to_string()
            };
            println!("  {:03}-{}: {}", s.number, s.slug, summary);
        }
    }

    println!("\nPlan: {}", config.plan_file);
    Ok(0)
}
