# Gastown issue to design mapping

This table maps recurring Gastown pain points to the Grit-backed Brat design and the remaining harness work.

## Issue to design table

| Pain point | Grit substrate response | Harness work needed |
| --- | --- | --- |
| Dirty working trees and phantom diffs | No tracked metadata; WAL in `refs/grit/wal` | Ensure harness never writes metadata into worktrees; export only on demand |
| Destructive or scary repair (doctor) | `brat doctor` is monotonic; rebuild projections only | Surface `doctor` and rebuild actions in the control room with clear prompts |
| Daemon required for correctness | Brat wraps a correct substrate; `gritd` is optional | Make harness tolerate daemon absence and fall back to CLI-only flows |
| Commands that hang (`brat feed`) | Grit CLI defaults to non-blocking; `--json` always available | Align harness UI and CLI to the same non-blocking contract |
| Orphaned sessions and stuck workers | Persistent actors + WAL event log | Implement session state machine + reconciliation on startup |
| Silent failures with no logs | Event log is durable; comments/labels can store exit data | Always post exit code + last logs to Grit comments on failure |
| Divergence / branch-topology heuristics | Merge by event union; deterministic projections | Ensure harness never uses branch state as coordination state |
| Config/flag drift | Grit has explicit config + actor selection | Add harness config schema + validation; show in control room |
| Engine portability issues | Grit is engine-agnostic | Implement engine trait adapters (Claude/Codex/OpenCode) |
| Fragile Beads/JQ tooling | Grit WAL is self-contained | Remove jq dependency; rely on structured events and CLI output |

## Detailed harness implementation work

### 1) Role lifecycle state machine

See `docs/state-machine.md` for the full specification. Summary:

- Session states: `spawned -> ready -> running -> handoff -> exit`
- Role states: `idle -> active -> degraded -> recovering`
- State transitions are idempotent and replayable

### 2) Session reconciliation on startup

On harness startup:

- List active sessions from the engine adapter
- List expected sessions from Grit comments/labels
- Reconcile differences:
  - Adopt orphaned sessions and post recovery notes
  - Mark missing sessions as exited with last known info

### 3) Observability contract

Every session must emit:

- Exit code
- Reason (signal, timeout, crash, user stop)
- Last N lines (or a hash + pointer)
- Timestamps (spawned, last heartbeat, exited)

All of the above are written as Grit comments, and summarized into labels for queryability.

### 4) Non-blocking UX contract

- All command entrypoints have a `--follow` or `--wait` opt-in
- Default outputs are bounded snapshots
- Control room uses polling or explicit follow modes

### 5) Engine abstraction

Implement a strict engine trait with timeouts and structured error reporting:

- `spawn`, `send`, `tail`, `stop`, `health`
- Adapters: Claude Code, Codex CLI, OpenCode, shell
- Standardize exit normalization across adapters

### 6) Worktree manager

- One worktree per agent
- Ensure `git status` stays clean for metadata
- Map worktrees to actors and post their paths in Grit comments

### 7) Merge/refinery pipeline

- Use Grit issues + labels to track merge state
- Make merge attempts explicit in comments
- Define merge queue policy and retry rules

### 8) Lock discipline

- Acquire `path:` locks for shared areas
- Use `repo:` locks for risky global operations
- Enforce lock policy when writing task updates

### 9) Control room UX (tmux)

- A canonical “cockpit” view shows:
  - active tasks
  - session health
  - merge queue
  - locks
- Control room commands are wrappers around Grit queries and harness actions
- Tmux naming convention:
  - Session: `brat`
  - Windows: `mayor`, `witness`, `refinery`, `deacon`, `sessions`

### 10) Compatibility and integration tests

Add an end-to-end suite that runs:

- convoy-like task bundle -> spawn swarm -> work -> merge
- concurrent agents -> event union -> deterministic projection
- `gritd` down -> CLI still correct
