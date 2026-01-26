# Usability upgrades (mixed teams)

This document defines the usability posture for Brat in mixed teams. Brat is the single CLI; Grite remains the substrate and stays hidden in day-to-day use.

## Principles

- Single CLI (`brat`) for all routine operations
- Convoy-first workflow with a clear single-task path
- Non-blocking defaults with explicit `--follow`/`--wait`
- Clear intervention points when the system needs help

## Default flows

### Convoy-first flow (default)

1. `brat init`
2. `brat convoy create --title ... --goal ...`
3. `brat task add --convoy <id> --title ... --paths ...`
4. `brat swarm start --n <count> --convoy <id>`
5. `brat status`
6. `brat refinery run --once` (or `bratd` loop)
7. `brat sync --push`

### Single-task flow (explicit)

1. `brat task add --solo --title ... --paths ...`
2. `brat task assign <task_id> --assignee <actor_id>`
3. `brat witness run --once`
4. `brat status`

`--solo` creates a one-task convoy behind the scenes and returns both IDs.

## Intervention points (what users fix)

Brat should surface these explicitly in `brat status` and the control room.

- **Stuck session**: no heartbeat for N minutes
  - Action: `brat session tail <id>`, `brat session stop <id>`, `brat witness run --once`
- **Task blocked**: `status:blocked` with missing resolution
  - Action: add a comment + update labels; optionally reassign
- **Merge failure**: `merge:failed` or failed checks
  - Action: `brat refinery run --once`, inspect logs, requeue
- **Lock conflict**: conflicting `path:` or `repo:` locks
  - Action: `brat lock status`, coordinate, or `--force`
- **Config error**: invalid or missing config
  - Action: `brat config validate`, fix `.brat/config.toml`
- **Daemon down**: `grited` or `bratd` unavailable
  - Action: restart; CLI should still function
- **Projection drift**: inconsistent local view
  - Action: `brat doctor --rebuild`

## Intervention messaging contract

Whenever Brat detects an intervention, it must include remediation instructions.

Human output requirements:

- A one-line summary of the issue
- The affected object (convoy/task/session)
- A suggested command (or commands) to remediate
- A cognitive prompt describing what information or decision is needed

JSON output requirements:

```
{
  "interventions": [
    {
      "kind": "stuck_session",
      "summary": "Session s-20250114-7b3d missed heartbeat for 5m",
      "task_id": "t-20250114-3a2c",
      "session_id": "s-20250114-7b3d",
      "cognitive_prompt": "Decide whether to wait, restart the session, or reassign the task. Add context if the task spec is unclear.",
      "recommended_actions": [
        "brat session tail s-20250114-7b3d --lines 200",
        "brat session stop s-20250114-7b3d",
        "brat witness run --once"
      ]
    }
  ]
}
```

## Human output example

```
Interventions needed:
- stuck_session: s-20250114-7b3d (task t-20250114-3a2c) missed heartbeat for 5m
  Cognitive prompt: Decide whether to wait, restart the session, or reassign the task. Add context if the task spec is unclear.
  Actions:
    brat session tail s-20250114-7b3d --lines 200
    brat session stop s-20250114-7b3d
    brat witness run --once
```

## Intervention catalog (standard prompts)

- `stuck_session`
  - Prompt: Decide whether to wait, restart the session, or reassign the task. Add context if the task spec is unclear.
  - Commands:
    - `brat session tail <session_id> --lines 200`
    - `brat session stop <session_id>`
    - `brat witness run --once`
- `blocked_task`
  - Prompt: Determine what information is missing (inputs, constraints, acceptance checks) and add it to the task.
  - Commands:
    - `brat task show <task_id> --json`
    - `brat task comment <task_id> --body \"<missing info>\"`
    - `brat task assign <task_id> --assignee <actor_id>`
- `merge_failed`
  - Prompt: Decide whether to fix conflicts, adjust the merge policy, or requeue the task.
  - Commands:
    - `brat task list --label merge:failed --json`
    - `brat refinery run --once`
- `lock_conflict`
  - Prompt: Decide whether to wait, coordinate with the lock holder, or force release.
  - Commands:
    - `brat lock status --json`
    - `brat lock release --resource \"path:<path>\"`
- `config_error`
  - Prompt: Identify the invalid setting and fix `.brat/config.toml`, then re-run the role loop.
  - Commands:
    - `brat config validate`
    - `brat witness run --once`
- `daemon_down`
  - Prompt: Decide whether to restart `bratd`/`grited` or proceed with CLI-only operations.
  - Commands:
    - `brat status --json`
    - `brat witness run --once`
- `projection_drift`
  - Prompt: Decide whether to rebuild local projections (`brat doctor --rebuild`) before proceeding.
  - Commands:
    - `brat doctor --rebuild`

## Default intervention thresholds

- Heartbeat interval: 30s
- Stale session: 5m without heartbeat
- Blocked task escalation: 24h in `status:blocked`
- Merge retry limit: 2 attempts

Defaults are configurable in `.brat/config.toml` under `[interventions]`, except merge retries which are under `[refinery]`.

## Status output expectations

`brat status` should show:

- Convoys and task counts by status
- Active sessions with last heartbeat
- Merge queue state and failures
- Lock conflicts
- A short “interventions needed” list with suggested commands

`brat status --watch` is the opt-in streaming mode used in the control room.

JSON schema details are defined in `docs/brat-status-schema.md`.

## Control room UX

- Tmux session: `brat`
- Windows: `mayor`, `witness`, `refinery`, `deacon`, `sessions`
- Each window highlights errors and intervention actions

## Example tmux layout

Session: `brat`

Window: `mayor`
```
Pane 0 (left): brat status --watch
Pane 1 (right-top): brat convoy list --json
Pane 2 (right-bottom): brat task list --json --label status:blocked
```

Window: `witness`
```
Pane 0 (left): brat session list --json
Pane 1 (right-top): brat session tail <session_id> --lines 200
Pane 2 (right-bottom): brat witness run --once
```

Window: `refinery`
```
Pane 0 (left): brat task list --label merge:queued --json
Pane 1 (right-top): brat task list --label merge:failed --json
Pane 2 (right-bottom): brat refinery run --once
```

Window: `deacon`
```
Pane 0 (left): brat lock status --json
Pane 1 (right-top): brat doctor --check --json
Pane 2 (right-bottom): brat sync --pull
```

Window: `sessions`
```
Pane 0 (left): brat session list --json
Pane 1 (right-top): brat task list --label status:running --json
Pane 2 (right-bottom): brat task list --label status:needs-review --json
```

## Bootstrap script

Use `docs/tmux-bootstrap.sh` to create the tmux session and windows.
