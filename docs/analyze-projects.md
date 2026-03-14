# Analysis of Claude Code Project Directories

## Overview

Claude Code stores session history in `~/.claude/projects/` using a path-based naming convention. This project has two directories due to a mid-stream rename from a typo.

## Directory Inventory

### 1. `-Users-mike-github-softwarewrighter-avoid-compation` (typo: missing 'c')

The original session directory, created when the project was initially named `avoid-compation`.

| Path | Size | Description |
|------|------|-------------|
| `4689ad2c-...ffbb....jsonl` | 929 KB | Main session JSONL (497 lines) |
| `4689ad2c-.../subagents/agent-a46eaa24fd4637823.jsonl` | 136 KB | Plan subagent (69 lines) |
| `4689ad2c-.../subagents/agent-a46eaa24fd4637823.meta.json` | 20 B | `{"agentType":"Plan"}` |

**Session ID**: `4689ad2c-ffbb-44bd-900d-10c7b051edca`
**Slug**: `immutable-strolling-candy`
**Time span**: 2026-03-14T18:12 to 18:51 UTC (~40 min)
**Turns**: 3 (turn_duration system messages at 18:27, 18:45, 18:51)
**Content**: 89 user messages, 122 assistant messages, 86 tool uses

This session contains the entire initial implementation:
- User described the problem (auto-compaction burns quota, loses context)
- Claude designed the tool (saga/step model)
- Project renamed from `avoid-compation` to `avoid-compaction` mid-session
- All source files written (13 files, ~550 lines of Rust)
- 35 integration tests added
- Pre-commit quality gates passed
- Committed as `feat: Initial implementation of avoid-compaction CLI tool`

The `cwd` field in system messages shifts from `.../avoid-compation` (turn 1) to `.../avoid-compaction` (turns 2-3), confirming the rename happened during the session.

### 2. `-Users-mike-github-softwarewrighter-avoid-compaction` (correct name)

The current session directory, created after the rename.

| Path | Size | Description |
|------|------|-------------|
| `0629f824-...a218....jsonl` | 324 KB | Main session JSONL (135 lines) |
| `0629f824-.../subagents/agent-adb45c09ae8e4bbcf.jsonl` | 247 KB | Explore subagent (162 lines) |
| `0629f824-.../subagents/agent-adb45c09ae8e4bbcf.meta.json` | 23 B | `{"agentType":"Explore"}` |
| `memory/` | (empty) | Auto memory directory |

**Session ID**: `0629f824-a218-46c9-bbc6-26c7a20d3152`
**Time span**: 2026-03-14T19:04 to 19:16 UTC (~12 min so far)
**Content**: 15 user messages, 17 assistant messages (this session -- /init and current analysis)

## JSONL Format Analysis

Each `.jsonl` file contains one JSON object per line. Key record types:

| Type | Description | Key Fields |
|------|-------------|------------|
| `user` | User message | `message.content`, `timestamp`, `cwd`, `sessionId`, `version` |
| `assistant` | Claude response | `message.content[]` (text blocks + tool_use blocks) |
| `progress` | Hook/tool progress | `data.type` (hook_progress, etc.) |
| `system` | System events | `subtype` (stop_hook_summary, turn_duration) |
| `file-history-snapshot` | File backup state | `snapshot.trackedFileBackups` |
| `queue-operation` | User queued messages | `operation` (enqueue/remove), `content` |
| `last-prompt` | Last user input | `lastPrompt` text |

### Key Observations

1. **JSONL is append-only**: Lines are never removed or rewritten. Even after compaction, the original messages remain in the JSONL file -- compaction happens in-memory by summarizing older messages, but the full history persists on disk.

2. **Subagents get separate files**: Each Agent tool invocation creates its own JSONL under `{sessionId}/subagents/agent-{id}.jsonl` with a `.meta.json` describing the agent type.

3. **Tool use is inline**: Assistant messages contain `tool_use` blocks within `message.content[]`, not as separate JSONL lines. Tool results appear as subsequent `user` messages with `tool_result` content blocks.

4. **Timestamps are UTC ISO 8601**: All timestamps use `Z` suffix (UTC).

5. **Session metadata**: Each `user` and `system` record includes `cwd`, `sessionId`, `version`, and `gitBranch`.

6. **Turn boundaries**: `system` records with `subtype: "turn_duration"` mark the end of each turn with `durationMs`.

## Implications for avoid-compaction

### Current Transcript Design (to be revised)

The current `--transcript` flag on `complete` expects the user to manually provide transcript text (from `/export` or typed). This is unreliable because:

- `/export` is not always available or convenient
- Manual copy-paste is error-prone
- The user may forget to export before context is lost

### Proposed: Direct JSONL Ingestion

The tool should read session history directly from `~/.claude/projects/`. The JSONL files are:

1. **Persistent** -- append-only, never truncated
2. **Comprehensive** -- contain all user messages, assistant responses, tool calls, and system events
3. **Structured** -- parseable JSON with timestamps, UUIDs, and parent-child relationships
4. **Available without user action** -- no need to run /export

### Revised Transcript Strategy

The `transcript` command (or a new `sync` command) should:

1. **Locate the project directory**: Derive from cwd path (replace `/` with `-`, prepend `-`)
2. **Find active sessions**: List `*.jsonl` files, identify the current/latest session
3. **Copy JSONL snapshots**: On each `complete`, copy the current session JSONL into `.avoid-compaction/` (or diff against last copy to store only new content)
4. **Preserve pre-compaction content**: Since the JSONL is append-only, earlier copies preserve the full conversation even if in-memory context was compacted. Diffing successive copies reveals what Claude "forgot" during compaction.
5. **Include subagent transcripts**: Copy subagent JSONL files alongside the main session

### Path Derivation

The Claude Code project directory path follows this pattern:
```
~/.claude/projects/{cwd_with_slashes_replaced_by_dashes}
```

Example:
```
/Users/mike/github/softwarewrighter/avoid-compaction
->  ~/.claude/projects/-Users-mike-github-softwarewrighter-avoid-compaction
```

Note the leading `-` (from the leading `/`).

### Multiple Sessions and Renames

As seen with this project, directory renames create new project directories. The tool should handle:
- Multiple project dirs mapping to the same repo (via rename detection or config)
- Multiple session IDs within a single project dir (one per `claude` invocation)
- Growing JSONL files within a session (append-only during conversation)
