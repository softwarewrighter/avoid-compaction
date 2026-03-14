use crate::error::Result;
use crate::read_input;
use crate::saga;
use std::path::Path;

pub fn run(saga_path: &Path, name: &str, plan: &str) -> Result<()> {
    let plan_content = read_input(plan)?;
    saga::init_saga(saga_path, name, &plan_content)?;
    println!("Saga '{name}' initialized at {}", saga_path.display());
    println!(
        "Plan written to {}",
        saga::saga_dir(saga_path).join("plan.md").display()
    );
    println!();
    println!("IMPORTANT for AI coding agents:");
    println!("  .avoid-compaction/ is an APPEND-ONLY history store.");
    println!("  Never delete, overwrite, or truncate any files in it.");
    println!("  The tool selectively shows you only what you need.");
    println!();
    println!("Workflow:");
    println!("  1. Run 'avoid-compaction next' at session start");
    println!("  2. Do the work described in the prompt");
    println!("  3. Run 'avoid-compaction complete --summary \"...\" \\");
    println!("       --next-slug <slug> --next-prompt \"...\" --next-context <files>'");
    println!("  4. Tell the user: \"You may Ctrl-C and restart.\"");
    Ok(())
}
