use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use avoid_compaction::commands;

#[derive(Parser)]
#[command(name = "avoid-compaction")]
#[command(
    about = "Session checkpoint tool for Claude Code — replace auto-compaction with deliberate context handoffs"
)]
#[command(long_about = "\
Manages sagas (sequences of focused steps) so you can restart Claude frequently \
without losing context. At the end of each session, save what happened and define \
the next step. On restart, retrieve the next step's prompt and context file list.

Typical workflow:
  1. avoid-compaction init --name my-feature --plan plan.md
  2. avoid-compaction next          # get current step prompt and context
  3. avoid-compaction begin         # mark step in-progress
  4. ... do the work ...
  5. avoid-compaction complete --summary \"what was done\" \\
       --next-slug add-tests --next-prompt prompt.md --next-context src/lib.rs
  6. Restart Claude, go to step 2

Session transcripts are captured automatically from Claude Code's JSONL session \
files at ~/.claude/projects/. No need to use /export.

Exit codes for 'next': 0 = step available, 1 = saga complete, 2 = no saga found.

AI CODING AGENT INSTRUCTIONS:

This tool manages multi-session context continuity for AI coding agents. Use it \
to checkpoint your work so the next session can pick up exactly where you left off.

SESSION START (run at the beginning of each session):
  avoid-compaction next
  Read the prompt and context files it outputs. These define your current task.

SESSION END (run when finishing a task or before context gets too large):
  avoid-compaction complete --summary \"Brief description of what was accomplished\" \\
    --next-slug <short-name> --next-prompt <file-or-text> \\
    --next-context src/foo.rs,src/bar.rs
  The session JSONL is automatically snapshotted into .avoid-compaction/sessions/.

FIRST SESSION (no steps exist yet):
  avoid-compaction init --name my-feature --plan \"Overall plan text\"
  avoid-compaction complete --next-slug first-step --next-prompt \"Do X\"

KEY BEHAVIORS:
  - 'complete' auto-snapshots the active ~/.claude/projects/ session JSONL
  - 'transcript' reads from JSONL snapshots, showing user/assistant conversation
  - 'history' shows summaries of all completed steps
  - 'list' outputs context file paths for the current step
  - '--done' on 'complete' marks the saga finished (no next step needed)
  - Steps follow a state machine: pending -> in-progress -> completed|blocked

DATA STORAGE:
  All data is in .avoid-compaction/ (TOML configs, markdown content, JSONL snapshots).
  Context files store paths only, not file contents -- read them yourself on restart.")]
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
