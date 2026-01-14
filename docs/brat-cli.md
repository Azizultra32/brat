# Brat CLI (harness)

The Brat CLI is the harness control plane. It orchestrates roles and uses Grit for all task/memory state.

## Principles

- Non-interactive by default
- Structured output via `--json`
- Safe by default; destructive actions require explicit flags
- Streaming and waiting require explicit flags (`--follow`, `--wait --timeout`)
- Single CLI: `brat` wraps Grit; `grit` is for debugging only

## Command surface (initial)

- `brat init`
- `brat status [--json] [--all-repos]`
- `brat convoy create --title ... --goal ...`
- `brat convoy create --mirror --repos <paths>`
- `brat convoy list [--json] [--all-repos]`
- `brat convoy show <convoy_id> [--json]`
- `brat convoy add-repo <convoy_id> --repo <path>`
- `brat task add --convoy <id> --title ... --paths ...`
- `brat task add --solo --title ... --paths ...`
- `brat task add --convoy <id> --repo <path> --title ...`
- `brat task assign <task_id> --assignee <actor_id>`
- `brat task list [--json] [--all-repos]`
- `brat task show <task_id> [--json]`
- `brat task comment <task_id> --body ...`
- `brat task close <task_id> --reason done`
- `brat swarm start --n <count> --convoy <id>`
- `brat swarm stop --convoy <id>`
- `brat session list [--json]`
- `brat session tail <session_id> --lines 200 [--json]`
- `brat session stop <session_id>`
- `brat witness run --once`
- `brat refinery run --once`
- `brat deacon run --once`
- `brat feed --once|--follow [--timeout <ms>]`
- `brat lock status [--json]`
- `brat lock acquire --resource <R> --ttl 15m`
- `brat lock release --resource <R>`
- `brat doctor --check|--rebuild`
- `brat sync --pull|--push`
- `brat export --format md|json`

## Output

- `--json` is supported on all read commands
- Errors return structured details
- `brat status --json` includes an `interventions` array with recommended remediation commands (see `docs/usability.md`).

## Relationship to Grit

Brat reads and writes Grit issues, comments, labels, and locks. It never writes tracked files for metadata. `brat init` wraps `grit init` for repo enablement.
