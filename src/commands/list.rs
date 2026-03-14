use crate::error::Result;
use crate::saga;
use crate::step;
use std::path::Path;

pub fn run(saga_path: &Path) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);

    if config.current_step == 0 {
        println!("No steps defined yet. No context files.");
        return Ok(());
    }

    let step_dir = step::find_step_dir(&saga_dir, config.current_step)?;
    let step_config = step::load_step(&step_dir)?;

    println!(
        "Context files for step {:03}-{}:",
        step_config.number, step_config.slug
    );
    if step_config.context_files.is_empty() {
        println!("  (none)");
    } else {
        for f in &step_config.context_files {
            println!("  {}", f);
        }
    }

    Ok(())
}
