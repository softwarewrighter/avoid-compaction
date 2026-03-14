use crate::error::Result;
use crate::saga;
use crate::step;
use std::path::Path;

pub fn run(saga_path: &Path) -> Result<()> {
    let config = saga::load_saga(saga_path)?;
    let saga_dir = saga::saga_dir(saga_path);
    let steps = step::list_steps(&saga_dir)?;

    println!("=== History: {} ===", config.name);
    println!("Status: {}\n", config.status);

    if steps.is_empty() {
        println!("No steps recorded yet.");
        return Ok(());
    }

    for (dir, s) in &steps {
        println!("{:03}-{} [{}]", s.number, s.slug, s.status);
        println!("  {}", s.description);

        let summary_path = dir.join("summary.md");
        if summary_path.is_file() {
            let content = std::fs::read_to_string(&summary_path)?;
            for line in content.lines().take(5) {
                println!("    {}", line);
            }
        }
        println!();
    }

    Ok(())
}
