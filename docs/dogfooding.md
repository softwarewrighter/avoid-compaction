# Dogfooding Log

Using avoid-compaction on its own development. This document tracks the dogfooding process, what works, what breaks, and what needs fixing.

## History

### Session 0 (2026-03-14, typo dir: avoid-compation)

The original session where the tool was conceived and built.

- User described the problem: auto-compaction burns quota, loses context, causes pauses
- Designed the saga/step model
- Renamed project from avoid-compation to avoid-compaction mid-session
- Implemented all 10 CLI commands (init, status, next, begin, complete, plan, transcript, history, list, abort)
- 13 source files, ~550 lines of Rust
- 35 integration tests
- Pre-commit quality gates passed
- Committed as initial implementation

### Session 1 (2026-03-14, current dir)

Continued development with /init, analysis, and feature work.

- Created CLAUDE.md for future sessions
- Analyzed ~/.claude/projects/ JSONL format (docs/analyze-projects.md)
- Discovered /export is unreliable; revised transcript strategy to use JSONL files directly
- Added session.rs module: JSONL discovery, parsing, snapshotting, diffing
- Added serde_json and dirs dependencies, 13 new session tests (48 total)
- Updated complete to auto-snapshot session JSONL on every completion
- Updated transcript to read from JSONL snapshots (with legacy fallback)
- Added AI CODING AGENT INSTRUCTIONS section to --help
- Rewrote next command with checklist-style output (plan, [x]/[ ] steps, full last summary, prompt, context files, "when done" instructions)
- Rewrote complete to print restart message ("You may Ctrl-C and restart")

## Current State

### What works

- init/complete/next/begin/status/history/transcript/list/abort commands
- Session JSONL auto-snapshot on complete
- Checklist output from next (plan, step list, task prompt, context files)
- Restart message from complete ("You may Ctrl-C and restart")
- --help with AI agent instructions

### What does not work yet

- **Step 0 is special but not handled well**: The first session (before any steps exist) has no structure. The agent reads a plan or creates one, does initial work, then calls complete to create step 1. But there is no prompt or context file mechanism for step 0 -- the user just types naturally.
- **No hook/slash-command integration**: The agent must remember to call the CLI manually. There is no SessionStart hook calling `next` or pre-exit hook calling `complete`.
- **Summary of step 0 work is lost**: When complete is called with current_step=0, the --summary flag has nowhere to attach (no step directory exists yet). The summary should be saved somewhere.
- **Agent doesn't know about the tool automatically**: The agent needs to be told (via CLAUDE.md, --help, or the plan) to use avoid-compaction. There is no automatic discovery.
- **No validation of prompt quality**: The agent writes free-text prompts for the next step. Nothing enforces that the prompt is sufficient for a cold-start agent.

### sw-checklist failures (being addressed via refactoring spikes)

- Module function count: session.rs (9), next.rs (12), complete.rs (10), step.rs (10) -- max 7
- Crate module count: 17 modules -- max 7
- Aggressive target: 4 fn/module, 4 modules/crate, scale outward

#### Refactoring Spike Sequence

Tech debt spikes inserted before feature work to reach compliance:

1. **Spike 1 -- merge-small-commands** (Low risk): Merge 7 single-function command modules into 2 (simple.rs + lifecycle.rs). Drops module count 17 to 12.
2. **Spike 2 -- extract-display** (Medium risk): Extract shared display helpers from next.rs and complete.rs into display.rs. Fixes function counts.
3. **Spike 3 -- create-workspace** (High risk): Convert to Cargo workspace, extract avoid-compaction-core crate (types, saga, step).
4. **Spike 4 -- extract-session-crate** (Medium risk): Extract avoid-compaction-session crate (path, parse, snapshot).
5. **Spike 5 -- reduce-binary-modules** (Medium risk): Final merge/extraction to reach 4 crates, each 3-4 modules, each 3-4 fn.

## Dogfooding Plan

### Round 1: Manual dogfooding (current)

Try the basic loop on this project:
1. Start fresh claude session
2. Agent calls `avoid-compaction next`
3. Agent does one step of work
4. Agent calls `avoid-compaction complete ...`
5. User restarts, repeat

Evaluate: Does the agent get enough context? Is the prompt good enough? What breaks?

### Round 2: Fix what broke

Address issues discovered in round 1. Likely:
- Step 0 handling (save initial context/summary)
- Prompt templates or structure
- Context file selection guidance

### Round 3: Hook integration

- Add SessionStart hook that runs `avoid-compaction next` automatically
- Add a reminder mechanism (CLAUDE.md instruction? hook?) for calling complete before exit
- Consider a slash command wrapper

### Round 4: Try on another project

Use avoid-compaction on a different Software Wrighter project to validate it works outside its own codebase.

## Observations

(To be filled in during dogfooding rounds)

### What the agent needs on cold start

- The plan (what is the overall goal?)
- What has been done (completed step summaries)
- What to do now (the prompt for this step)
- Which files to read (context_files)
- How to signal completion (the complete command)

### What the agent needs to produce before exit

- Summary of what was accomplished
- Next step slug, prompt, and context files
- The complete command must succeed (JSONL snapshot, step transition)

### Open questions

- Should the tool auto-detect the saga from cwd, or require explicit init?
- How much of the plan should be in the saga vs in CLAUDE.md?
- Should context_files include the plan and prior summaries, or are those always shown by next?
- Is the step granularity right? Too coarse = context bloat. Too fine = overhead.
