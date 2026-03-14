# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**avoid-compaction** is a Rust CLI tool that replaces uncontrolled auto-compaction in Claude Code with deliberate context handoffs. It manages "sagas" (sequences of focused steps) so Claude can be restarted frequently without losing context. Data is stored in `.avoid-compaction/` using TOML configs and Markdown content files.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test test_name           # Run a single test
cargo test -- --nocapture      # Tests with stdout
cargo clippy --all-targets --all-features -- -D warnings  # Lint (zero warnings enforced)
cargo fmt --all                # Format code
cargo fmt --check              # Verify formatting
```

## Pre-Commit Quality Gates (Mandatory)

All steps must pass in order before every commit:

1. `cargo test` -- all tests pass, no disabled tests
2. `cargo clippy --all-targets --all-features -- -D warnings` -- zero warnings, never use `#[allow(...)]`
3. `cargo fmt --all` then `cargo fmt --check`
4. `markdown-checker -f "**/*.md"` -- ASCII-only markdown
5. `git status` -- verify no unintended files
6. `sw-checklist` -- project compliance check
7. Update docs if changes warrant it

Tools `markdown-checker` and `sw-checklist` are in `~/.local/softwarewrighter/bin/`. All support `--help` with AI-specific guidance.

## Architecture

- **`src/main.rs`** -- CLI entry point using `clap` derive macros
- **`src/lib.rs`** -- Public types (`SagaConfig`, `StepConfig`, `SagaStatus`, `StepStatus`), `read_input()` utility, timestamp helpers
- **`src/saga.rs`** -- Saga file I/O: `init_saga`, `load_saga`, `save_saga`
- **`src/step.rs`** -- Step file I/O and state machine: `create_step`, `transition_step`, `list_steps`
- **`src/error.rs`** -- `thiserror`-derived error enum
- **`src/commands/`** -- One module per CLI subcommand (`init`, `status`, `next`, `begin`, `complete`, `plan`, `transcript`, `history`, `list`, `abort`)
- **`tests/`** -- Integration tests: `saga_tests.rs`, `step_tests.rs`, `command_tests.rs` (35+ tests using `tempfile` for isolation)

### Step Status State Machine

`Pending` -> `InProgress` (begin) -> `Completed` (complete) or `Blocked` (abort). No other transitions allowed.

### Data Layout

```
.avoid-compaction/
  saga.toml              # name, status, current_step, created_at
  plan.md                # saga plan text
  steps/NNN-slug/        # e.g. 001-add-routes
    step.toml            # number, slug, status, description, context_files
    prompt.md            # next step prompt
    summary.md           # completion summary
  YYYYMMDDTHHMMSS-transcript.txt   # legacy manual transcripts
  sessions/                        # JSONL snapshots from ~/.claude/projects/
```

## Transcript Strategy

`/export` is unreliable. Instead, transcripts come from Claude Code's own session JSONL files at `~/.claude/projects/{path-with-dashes}/`. These are append-only and survive compaction. The tool should:

1. Derive the project dir from cwd (replace `/` with `-`, prepend `-`)
2. Copy/diff the active session JSONL into `.avoid-compaction/sessions/` on each `complete`
3. Preserve pre-compaction content by diffing successive snapshots
4. Include subagent JSONL files alongside the main session

See `docs/analyze-projects.md` for the full JSONL format analysis.

## Code Style

- Rust 2024 edition
- Inline format args: `format!("{name}")` not `format!("{}", name)`
- Module docs use `//!`, item docs use `///`
- Files < 500 lines (prefer 200-300), functions < 50 lines (prefer 10-30)
- Max 3 TODOs per file, no FIXMEs in commits

## Ratchet Rule (sw-checklist)

`sw-checklist` measures project compliance. The current baseline is **7 passed, 11 failed, 24 warnings**. When adding code, you must not make this worse -- maintain or improve the counts. Take a baseline before starting work (`sw-checklist 2>&1 | tail -1`) and verify after. This is a ratchet: it only turns one way.

## TDD Workflow

This project uses strict Red/Green/Refactor TDD. Write a failing test first, make it pass with minimal code, then refactor.

## Commit Convention

```
type: Short summary (50 chars max)

Detailed explanation of what and why.

Co-Authored-By: Claude <noreply@anthropic.com>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`. Push immediately after every commit.
