# avoid-compaction: Explain Like I'm 5

## The Problem

Claude Code has a context window. When a conversation gets too long,
Claude automatically compacts (summarizes) older messages to make room.
This loses detail. The AI forgets what it was doing, what decisions were
made, and what comes next. You end up re-explaining things or watching
the agent retrace steps it already took.

Restarting Claude gives you a fresh, full context window -- but now the
agent knows nothing at all.

## The Idea

Instead of letting compaction silently erase context, **restart early
and hand off deliberately**. At the end of each session, the agent
writes down:

- What it accomplished (summary)
- What the next agent should do (prompt)
- Which files the next agent should read (context)
- What the overall plan looks like now (plan)

The next session starts by reading those notes. It gets exactly the
context it needs -- no more, no less -- and a full context window to
work with.

## The Metaphor

Think of a relay race. Each runner (session) carries a baton (context).
The handoff zone is where `complete` passes the baton to `next`. A
clean handoff means the next runner hits full speed immediately. A
dropped baton (compaction) means stopping to figure out what happened.

## Phases of Use

### Phase 1: Starting a New Project

You have an idea. Maybe some planning docs, a PRD, or just a paragraph.
You start Claude and tell it to read those docs and run
`avoid-compaction --help`.

The agent sees "exit code 2 -- no saga exists" and knows this is the
beginning. It focuses on **planning, not implementation**:

- Reads your planning documents
- Creates the saga: `avoid-compaction init --name my-feature --plan ...`
- Defines the first real step: `avoid-compaction complete --next-slug ...`
- Tells you to restart

**What the agent should NOT do**: start coding. Step 0 is for planning.

### Phase 2: The Session Loop (Most of Your Time)

Every session after the first follows the same rhythm:

```
avoid-compaction next      <-- "what should I do?"
... do the work ...        <-- implement, fix, spike, refactor
avoid-compaction plan      <-- review/revise the plan if needed
git commit                 <-- save the code (mandatory!)
avoid-compaction complete  <-- record what happened, define next step
```

Then you restart Claude and paste `avoid-compaction next`. The new
agent picks up exactly where the last one left off.

Each step is a focused unit of work: one feature, one bug fix, one
spike. Small enough to finish in a single session. If a step is too
big, the agent should break it down during the `plan` phase.

### Phase 3: The Plan Evolves

The plan is not fixed. Every session, the agent should review it:

- Did this step reveal something unexpected?
- Does the order of remaining steps still make sense?
- Should new steps be added? Old ones dropped?

`avoid-compaction plan --update` captures the revised plan.
`--planned` flags on `complete` list the upcoming steps so the next
agent can see the roadmap.

### Phase 4: Saga Complete

When the last step is done, the agent uses `--done` instead of
`--next-slug`. The saga is marked complete.

But "complete" is not permanent. If more work surfaces later (bug
reports, new requirements, compliance fixes), the agent can reopen the
saga with `complete --next-slug` and keep going. No need to start over.

### Phase 5: Starting Another Saga

Different feature? Different project? Just run `init` in a new
directory. Each saga is independent. Each directory gets its own
`.avoid-compaction/` folder.

## What Gets Stored

Everything lives in `.avoid-compaction/` alongside your code:

```
.avoid-compaction/
  saga.toml                    # name, status, current step
  plan.md                      # the evolving plan
  planned-steps.md             # upcoming steps the agent foresees
  steps/001-add-routes/
    step.toml                  # status, description, context files
    prompt.md                  # what the agent was told to do
    summary.md                 # what the agent actually did
  steps/002-add-tests/
    ...
  sessions/
    <session-id>.jsonl         # snapshots of Claude's session logs
```

This is append-only. Steps accumulate. Summaries accumulate. You can
always look back at what happened and why.

## The Commands, In Order of Importance

| Command    | When           | What it does                              |
|------------|----------------|-------------------------------------------|
| `next`     | Session start  | Shows the current step's prompt + context |
| `complete` | Session end    | Records summary, creates next step        |
| `plan`     | Mid-session    | View or update the saga's plan            |
| `init`     | Once per saga  | Creates a new saga                        |
| `status`   | Anytime        | Shows saga state at a glance              |
| `history`  | Anytime        | Shows all completed step summaries        |

## Why Not Just Use CLAUDE.md?

CLAUDE.md is static project guidance. It tells the agent how to behave
in this codebase (style, testing, tooling). avoid-compaction is dynamic
session state. It tells the agent what to do right now, what was done
before, and what comes next. They complement each other.

## Why Not Just Use Longer Context?

Longer context delays the problem; it does not solve it. A 200k-token
conversation still compacts eventually. And long conversations degrade
in other ways: the agent gets slower, less focused, more prone to
contradicting earlier decisions. Frequent restarts with clean handoffs
keep every session sharp.

## One-Line Summary

avoid-compaction turns "one long conversation that forgets" into "many
short conversations that remember."
