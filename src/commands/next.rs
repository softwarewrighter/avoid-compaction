use crate::error::Result;
use crate::{StepStatus, saga, step, truncate};
use std::path::Path;

/// Return the command prefix for invoking this tool.
/// Uses the binary name if installed, falls back to `cargo run --quiet --`.
pub fn cmd_prefix() -> String {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join("avoid-compaction").is_file() {
                return "avoid-compaction".to_string();
            }
        }
    }
    "cargo run --quiet --".to_string()
}

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

    print_checklist(
        &config.name,
        &plan_content,
        &steps,
        &step_config,
        &step_dir,
        &saga_dir,
    )
}

fn print_first_step(saga_name: &str, plan: &str) {
    let cmd = cmd_prefix();
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
    println!("--- Setup ---");
    println!("  Install the SessionStart hook so future sessions get context automatically:");
    println!("  {cmd} install-hook");
    println!();
    println!("--- Checklist ---");
    println!("  [ ] Run: {cmd} install-hook");
    println!("  [ ] Review the plan above");
    println!("  [ ] Do the first unit of work");
    println!("  [ ] Run: {cmd} complete --summary \"...\" \\");
    println!("        --next-slug <slug> --next-prompt \"...\" --next-context <files>");
    println!();
    println!("  Then tell the user: \"Restart and give the new agent:");
    println!("    {cmd} next\"");
}

fn print_checklist(
    saga_name: &str,
    plan: &str,
    steps: &[(std::path::PathBuf, crate::StepConfig)],
    current: &crate::StepConfig,
    current_dir: &Path,
    saga_dir: &Path,
) -> Result<u8> {
    println!("=== Saga: {saga_name} ===");
    println!();

    if !plan.is_empty() {
        println!("--- Plan ---");
        println!("{plan}");
        println!();
    }

    print_step_list(steps, current);
    print_planned_steps(saga_dir);
    print_step0_summary(steps, saga_dir)?;
    print_last_completed_summary(steps)?;
    print_current_prompt(current, current_dir)?;
    print_context_files(current);
    print_when_done(current);

    Ok(0)
}

fn print_step_list(steps: &[(std::path::PathBuf, crate::StepConfig)], current: &crate::StepConfig) {
    println!("--- Steps ---");
    for (dir, s) in steps {
        if s.status == StepStatus::Completed {
            let summary = truncate(&read_summary(dir), 72);
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
        println!();
    }
}

fn print_step0_summary(
    steps: &[(std::path::PathBuf, crate::StepConfig)],
    saga_dir: &Path,
) -> Result<()> {
    let step0_summary_path = saga_dir.join("step0-summary.md");
    if step0_summary_path.is_file() {
        let has_completed_steps = steps.iter().any(|(_, s)| s.status == StepStatus::Completed);
        if !has_completed_steps {
            let content = std::fs::read_to_string(&step0_summary_path)?;
            println!("--- Prior session (step 0) ---");
            println!("{content}");
            println!();
        }
    }
    Ok(())
}

fn print_last_completed_summary(steps: &[(std::path::PathBuf, crate::StepConfig)]) -> Result<()> {
    let last_completed = steps
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
    Ok(())
}

fn print_current_prompt(current: &crate::StepConfig, current_dir: &Path) -> Result<()> {
    let prompt_path = current_dir.join("prompt.md");
    if prompt_path.is_file() {
        println!(
            "--- Your task (step {:03}-{}) ---",
            current.number, current.slug
        );
        println!("{}", std::fs::read_to_string(&prompt_path)?);
        println!();
    }
    Ok(())
}

fn print_context_files(current: &crate::StepConfig) {
    if !current.context_files.is_empty() {
        println!("--- Read these files for context ---");
        for f in &current.context_files {
            println!("  {f}");
        }
        println!();
    }
}

fn print_when_done(current: &crate::StepConfig) {
    let cmd = cmd_prefix();
    println!("--- When done ---");
    println!("  1. Run pre-commit gates: cargo test, cargo clippy, cargo fmt");
    println!("  2. Commit changes with a detailed message");
    println!("  3. Run: {cmd} complete --summary \"...\" \\");
    println!("       --next-slug <slug> --next-prompt \"...\" --next-context <files>");
    println!("     Or: {cmd} complete --summary \"...\" --done");
    println!();
    println!(
        "  Then tell the user: \"Step {:03} complete. Restart and give the new agent:",
        current.number
    );
    println!("    {cmd} next\"");
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
