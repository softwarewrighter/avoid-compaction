use crate::error::Result;
use crate::{StepStatus, saga, step};
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
    let plan_path = saga_path.join(&config.plan_file);
    let plan_content = if plan_path.is_file() {
        std::fs::read_to_string(&plan_path)?
    } else {
        String::new()
    };

    if config.current_step == 0 {
        print_first_step(&config.name, &plan_content);
        return Ok(0);
    }

    let steps = step::list_steps(&saga_dir)?;
    let step_dir = step::find_step_dir(&saga_dir, config.current_step)?;
    let step_config = step::load_step(&step_dir)?;

    print_checklist(&config.name, &plan_content, &steps, &step_config, &step_dir)
}

fn print_first_step(saga_name: &str, plan: &str) {
    println!("=== Saga: {saga_name} ===");
    println!("Status: FIRST STEP (no prior work)");
    println!();
    if !plan.is_empty() {
        println!("--- Plan ---");
        println!("{plan}");
        println!();
    }
    println!("--- Note ---");
    println!("  .avoid-compaction/ is an APPEND-ONLY history store.");
    println!("  Never delete, overwrite, or truncate any files in it.");
    println!("  The tool selectively shows you only what you need.");
    println!();
    println!("--- Checklist ---");
    println!("  [ ] Review the plan above");
    println!("  [ ] Do the first unit of work");
    println!("  [ ] Run: avoid-compaction complete --summary \"...\" \\");
    println!("        --next-slug <slug> --next-prompt \"...\" --next-context <files>");
    println!();
    println!("When complete is done, tell the user they may Ctrl-C and restart.");
}

fn print_checklist(
    saga_name: &str,
    plan: &str,
    steps: &[(std::path::PathBuf, crate::StepConfig)],
    current: &crate::StepConfig,
    current_dir: &Path,
) -> Result<u8> {
    println!("=== Saga: {saga_name} ===");
    println!();

    // Plan
    if !plan.is_empty() {
        println!("--- Plan ---");
        println!("{plan}");
        println!();
    }

    // Step checklist: completed steps get [x], current gets [ ]
    println!("--- Steps ---");
    for (dir, s) in steps {
        if s.status == StepStatus::Completed {
            let summary = read_summary(dir);
            println!("  [x] {:03}-{}: {}", s.number, s.slug, summary);
        } else if s.number == current.number {
            println!(
                "  [ ] {:03}-{}: {} <-- YOU ARE HERE",
                s.number, s.slug, s.description
            );
        } else {
            println!("  [ ] {:03}-{}: {}", s.number, s.slug, s.description);
        }
    }
    println!();

    // Full summary of the most recent completed step
    let last_completed: Option<&(std::path::PathBuf, crate::StepConfig)> = steps
        .iter()
        .rev()
        .find(|(_, s)| s.status == StepStatus::Completed);

    if let Some((dir, s)) = last_completed {
        let summary_path = dir.join("summary.md");
        if summary_path.is_file() {
            let content = std::fs::read_to_string(&summary_path)?;
            println!("--- Last completed: {:03}-{} ---", s.number, s.slug);
            println!("{content}");
            println!();
        }
    }

    // Current step prompt
    let prompt_path = current_dir.join("prompt.md");
    if prompt_path.is_file() {
        println!(
            "--- Your task (step {:03}-{}) ---",
            current.number, current.slug
        );
        println!("{}", std::fs::read_to_string(&prompt_path)?);
        println!();
    }

    // Context files to read
    if !current.context_files.is_empty() {
        println!("--- Read these files for context ---");
        for f in &current.context_files {
            println!("  {f}");
        }
        println!();
    }

    // What to do when done
    println!("--- When done ---");
    println!("  Run: avoid-compaction complete --summary \"...\" \\");
    println!("    --next-slug <slug> --next-prompt \"...\" --next-context <files>");
    println!("  Or: avoid-compaction complete --summary \"...\" --done");
    println!(
        "  Then tell the user: \"Step {:03} complete. You may Ctrl-C and restart.\"",
        current.number
    );

    Ok(0)
}

fn read_summary(step_dir: &Path) -> String {
    let summary_path = step_dir.join("summary.md");
    if summary_path.is_file()
        && let Ok(content) = std::fs::read_to_string(&summary_path)
    {
        return content.lines().next().unwrap_or("(done)").to_string();
    }
    "(no summary)".to_string()
}
