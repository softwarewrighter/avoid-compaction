use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use avoid_compaction::commands;

#[derive(Parser)]
#[command(name = "avoid-compaction", version)]
#[command(
    about = "Session checkpoint tool for Claude Code — replace auto-compaction with deliberate context handoffs"
)]
#[command(long_about = "\
Manages sagas (sequences of focused steps) so you can restart Claude frequently \
without losing context. Each session follows the same three-command loop:

  avoid-compaction next       # what should I do?
  ... do the work ...
  avoid-compaction plan       # review/update the plan
  avoid-compaction complete   # record what happened, define next step

Then the user restarts Claude and pastes the command shown by 'complete'.

AI CODING AGENT INSTRUCTIONS:

ALWAYS start every session by running:
  avoid-compaction next

This command has three possible outcomes:

  Exit code 0 -- A step is ready. Read the prompt and context files it outputs. \
Execute that step (implement a feature, fix a bug, run a spike, etc.). When done, \
review/update the plan with 'avoid-compaction plan', then run 'complete'.

  Exit code 2 -- No saga exists yet. This is the BEGINNING of a new project. \
Your job is to focus on PLANNING, not implementation. Read any planning documents \
the user has provided, understand the project goals, then:
    avoid-compaction init --name <saga-name> --plan <plan-text-or-file>
  Then run 'complete' to define the first implementation step.

  Exit code 1 -- The saga is finished. Tell the user: \"All steps are complete. \
The saga is done.\" Do NOT tell them to restart.

SESSION LOOP (every session, every step):
  1. avoid-compaction next
  2. Do the work described by the step prompt
  3. avoid-compaction plan --update <revised-plan>   (if the plan needs changes)
  4. Run ALL pre-commit quality gates (tests, lints, formatting)
  5. git commit -- you MUST commit before running complete
  6. avoid-compaction complete --summary \"what was done\" \\
       --next-slug <slug> --next-prompt \"what to do next\" \\
       --next-context file1.rs,file2.rs \\
       --planned \"future-slug: description\" \\
       --planned \"another-slug: another description\"
  7. Tell the user: \"Step N complete. Restart Claude and run:
       avoid-compaction next\"

CRITICAL: Steps 4-5 are mandatory. NEVER run 'complete' with uncommitted changes. \
The commit preserves your work; 'complete' only records metadata for the next agent. \
If you skip the commit, the next agent has no code to build on.

IMPORTANT -- always use --planned with 'complete' to list upcoming steps you can \
foresee. This gives future agents visibility into the roadmap. If all work is \
done, use --done instead of --next-slug.

COMPLETE COMMAND REFERENCE:
  --summary    What you accomplished this step (required)
  --next-slug  Short name for the next step, e.g. \"add-api-routes\"
  --next-prompt  Instructions for the next agent (text, file path, or \"-\" for stdin)
  --next-context  Comma-separated file paths the next agent should read
  --planned    Repeatable. Future steps as \"slug: description\"
  --done       No more steps -- marks the saga as complete

OTHER COMMANDS:
  status      Show current saga state
  history     Show all completed step summaries
  transcript  View session transcript for a step
  list        List context files for the current step
  plan        View or update the saga plan
  abort       Mark current step as blocked (with optional --reason)

DATA STORAGE:
  All data is in .avoid-compaction/ (TOML configs, markdown content, JSONL snapshots). \
Context files store paths only -- read them yourself on restart.")]
struct Cli {
    /// Path to the saga's project directory (default: current directory)
    #[arg(long, default_value = ".")]
    saga: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new saga with a name and plan
    Init {
        /// Name for the saga
        #[arg(long)]
        name: String,

        /// Plan file path, literal text, or "-" for stdin
        #[arg(long)]
        plan: String,
    },

    /// Show current saga state
    Status,

    /// Output the next step's prompt and context file list (for Claude on restart)
    Next,

    /// Mark the current step as in-progress
    Begin,

    /// Complete current step: save transcript/summary, define next step
    Complete {
        /// Transcript text, file path, or "-" for stdin
        #[arg(long)]
        transcript: Option<String>,

        /// Summary text, file path, or "-" for stdin
        #[arg(long)]
        summary: Option<String>,

        /// Prompt for the next step: text, file path, or "-" for stdin
        #[arg(long)]
        next_prompt: Option<String>,

        /// Short slug for the next step (e.g., "add-api-routes")
        #[arg(long)]
        next_slug: Option<String>,

        /// Comma-separated list of context file paths for the next step
        #[arg(long, value_delimiter = ',')]
        next_context: Vec<String>,

        /// Planned future steps (repeatable), each "slug: description"
        #[arg(long)]
        planned: Vec<String>,

        /// Mark the saga as complete (no next step)
        #[arg(long)]
        done: bool,
    },

    /// View or update the saga's plan
    Plan {
        /// Update plan from file path, literal text, or "-" for stdin
        #[arg(long)]
        update: Option<String>,
    },

    /// View transcript for a step (default: most recent)
    Transcript {
        /// Step number to show transcript for
        #[arg(long)]
        step: Option<u32>,
    },

    /// Show all step summaries in order
    History,

    /// List context files for the current step
    List,

    /// Mark the current step as blocked
    Abort {
        /// Reason for blocking
        #[arg(long)]
        reason: Option<String>,
    },
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "-V" || args[1] == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!();
        println!("Copyright (c) 2026 Michael A Wright");
        println!("MIT License");
        println!();
        println!("Repository: https://github.com/softwarewrighter/avoid-compaction");
        println!("Build Commit: {}", env!("BUILD_COMMIT"));
        println!("Build Host: {}", env!("BUILD_HOST"));
        println!("Build Time: {}", env!("BUILD_TIME"));
        return ExitCode::SUCCESS;
    }

    let cli = Cli::parse();

    match dispatch(&cli.saga, cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::from(1)
        }
    }
}

fn dispatch(saga_path: &std::path::Path, command: Commands) -> avoid_compaction::error::Result<u8> {
    match command {
        Commands::Init { name, plan } => commands::init::run(saga_path, &name, &plan).map(|_| 0u8),
        Commands::Status => commands::status::run(saga_path).map(|_| 0),
        Commands::Next => commands::next::run(saga_path),
        Commands::Begin => commands::begin::run(saga_path).map(|_| 0),
        Commands::Complete {
            transcript,
            summary,
            next_prompt,
            next_slug,
            next_context,
            planned,
            done,
        } => {
            let args = commands::complete::CompleteArgs {
                transcript: transcript.as_deref(),
                summary: summary.as_deref(),
                next_prompt: next_prompt.as_deref(),
                next_slug: next_slug.as_deref(),
                next_context,
                planned,
                done,
            };
            commands::complete::run(saga_path, &args).map(|_| 0)
        }
        Commands::Plan { update } => commands::plan::run(saga_path, update.as_deref()).map(|_| 0),
        Commands::Transcript { step } => commands::transcript::run(saga_path, step).map(|_| 0),
        Commands::History => commands::history::run(saga_path).map(|_| 0),
        Commands::List => commands::list::run(saga_path).map(|_| 0),
        Commands::Abort { reason } => commands::abort::run(saga_path, reason.as_deref()).map(|_| 0),
    }
}
