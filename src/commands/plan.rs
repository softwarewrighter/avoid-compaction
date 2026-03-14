use crate::error::Result;
use crate::{read_input, saga};
use std::path::Path;

pub fn run(saga_path: &Path, update: Option<&str>) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let plan_path = saga_path.join(&config.plan_file);

    if let Some(update_val) = update {
        let content = read_input(update_val)?;
        std::fs::write(&plan_path, &content)?;
        println!("Plan updated: {}", plan_path.display());
    } else if plan_path.is_file() {
        println!("{}", std::fs::read_to_string(&plan_path)?);
    } else {
        println!("No plan file found at {}", plan_path.display());
    }

    Ok(())
}
