use crate::error::Result;
use crate::saga;
use crate::step;
use std::path::Path;

pub fn run(saga_path: &Path) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);
    let steps = step::list_steps(&saga_dir)?;

    let completed = steps
        .iter()
        .filter(|(_, s)| s.status == crate::StepStatus::Completed)
        .count();

    println!("Saga: {}", config.name);
    println!("Status: {}", config.status);
    println!("Steps: {} total, {} completed", steps.len(), completed);
    println!("Current step: {}", config.current_step);
    println!("Plan: {}", config.plan_file);

    if config.current_step > 0
        && let Ok(dir) = step::find_step_dir(&saga_dir, config.current_step)
        && let Ok(step_config) = step::load_step(&dir)
    {
        println!(
            "\nCurrent: {:03}-{} [{}]",
            step_config.number, step_config.slug, step_config.status
        );
        println!("  {}", step_config.description);
    }

    Ok(())
}
