use crate::error::Result;
use crate::read_input;
use crate::saga;
use std::path::Path;

pub fn run(saga_path: &Path, name: &str, plan: &str) -> Result<()> {
    let plan_content = read_input(plan)?;
    saga::init_saga(saga_path, name, &plan_content)?;
    println!("Saga '{}' initialized at {}", name, saga_path.display());
    println!(
        "Plan written to {}",
        saga::saga_dir(saga_path).join("plan.md").display()
    );
    Ok(())
}
