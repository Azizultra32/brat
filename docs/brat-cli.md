# Brat CLI (harness)

The Brat CLI is the harness control plane. It orchestrates roles and uses Grite for all task/memory state.

## Principles

- Non-interactive by default
- Structured output via `--json`
- Safe by default; destructive actions require explicit flags
- Streaming and waiting require explicit flags (`--follow`, `--wait --timeout`)
- Single CLI: `brat` wraps Grite; `grite` is for debugging only
- Daemon auto-starts on commands that benefit from it (disable with `--no-daemon`)

## Global flags

- `--json` - Output in JSON format
- `--quiet` - Suppress human-readable output
- `--repo <path>` - Target a specific repository
- `--no-daemon` - Don't auto-start the daemon (run in standalone mode)

## Repo scope

- Commands operate on the current repo by default.
- Use `--repo <path>` to target a different repo.
- Use `--all-repos` on list/status commands to aggregate across repos.
- All CLI commands connect to the single `bratd` session registry when it is running.

## Command surface

- `brat init [--no-daemon] [--no-tmux] [--no-config]`
- `brat status [--json] [--all-repos] [--convoy <convoy_id>] [--watch]`
- `brat convoy create --title ... [--body ...]`
- `brat task create --convoy <id> --title ... [--body ...]`
- `brat task update <task_id> --status <queued|running|blocked|needs-review|merged|dropped> [--force]`
- `brat task dep add <task_id> --target <task_id> [--dep-type depends_on|blocks|related_to]`
- `brat task dep remove <task_id> --target <task_id> [--dep-type depends_on|blocks|related_to]`
- `brat task dep list <task_id> [--reverse]`
- `brat task dep topo [--convoy <convoy_id>]`
- `brat context index [--path <path>] [--pattern <glob>] [--force]`
- `brat context query <query>`
- `brat context show <path>`
- `brat context project [key]`
- `brat context set <key> <value>`
- `brat session list [--task <task_id>] [--json]`
- `brat session show <session_id> [--json]`
- `brat session tail <session_id> --lines 200 [--json]`
- `brat session stop <session_id> [--reason ...]`
- `brat witness run --once`
- `brat refinery run --once`
- `brat lock status [--json]`
- `brat doctor --check|--rebuild` (see note below)
- `brat workflow list|show|run`
- `brat mayor start|ask|status|tail|stop`
- `brat daemon start [--port <port>] [--idle-timeout <secs>] [--foreground]`
- `brat daemon stop`
- `brat daemon status [--json]`
- `brat daemon restart [--port <port>] [--idle-timeout <secs>]`
- `brat daemon logs [-n <lines>]`
- `brat api [--host <host>] [--port <port>] [--idle-timeout <secs>]` (deprecated, use `brat daemon start --foreground`)

## Doctor command

- `brat doctor --check`: read-only harness health validation
- `brat doctor --check --json`: includes `gritee_projection_accessible` for CLI-only Grite projection access and `gritee_db_maintenance` for DB stats/rebuild recommendations
- `brat doctor --rebuild`: reconciles Brat harness state such as stale sessions and abandoned worktrees

For substrate-level health checks and local Grite projection repair, use
`grite doctor --json`, `grite doctor --fix --json`, or `grite rebuild`
directly. These operations repair local projections; they must not rewrite
`refs/grite/*` or tracked project files.

Recommended DB lock recovery ladder:

1. `brat --no-daemon doctor --check --json`
2. `brat daemon status --json`
3. `grite daemon status --json`
4. `grite daemon stop` if the Grite daemon is stale
5. `grite doctor --fix --json` or `grite rebuild` if the local projection remains unusable

Recommended DB maintenance signals:

1. Run `brat --no-daemon doctor --check --json` and inspect `gritee_db_maintenance`.
2. Run `grite --no-daemon db stats --json` for `size_bytes`, `events_since_rebuild`, `days_since_rebuild`, and `rebuild_recommended`.
3. Rebuild when `rebuild_recommended` is `true`, events/days since rebuild cross the operations thresholds, or the projection remains unreadable after daemon recovery.

## Output

- `--json` is supported on all read commands
- Errors return structured details
- `brat status --json` includes an `interventions` array with recommended remediation commands (see `docs/usability.md`).
- `brat status --json --all-repos` returns a `repos` array (see `docs/brat-status-schema.md`).

## Relationship to Grite

Brat reads and writes Grite issues, comments, labels, and locks. It never writes tracked files for metadata.

`brat init` behavior:

- Initializes the Grite ledger (equivalent to `grite init`)
- Creates `.brat/config.toml` if missing (unless `--no-config`)
- Starts `bratd` unless `--no-daemon`
- Optionally creates the tmux control room (unless `--no-tmux`)
