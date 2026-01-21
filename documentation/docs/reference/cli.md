# CLI Reference

Complete reference for the Brat command-line interface.

## Global Flags

Available on all commands:

| Flag | Description |
|------|-------------|
| `--json` | Output in JSON format |
| `--quiet` | Suppress human-readable output |
| `--repo <path>` | Target a specific repository |
| `--no-daemon` | Don't auto-start the daemon |
| `--verbose` | Verbose output |
| `--help` | Show help |
| `--version` | Show version |

## Initialization

### `brat init`

Initialize Brat harness in the current repository.

```bash
brat init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--no-daemon` | Don't start daemon |
| `--no-tmux` | Don't create tmux session |
| `--no-config` | Don't create config file |

Behavior:

- Initializes Grit ledger (`grit init`)
- Creates `.brat/config.toml` (unless `--no-config`)
- Starts daemon (unless `--no-daemon`)

## Status

### `brat status`

View convoys, tasks, and sessions.

```bash
brat status [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |
| `--all-repos` | Aggregate across repos |
| `--convoy <id>` | Filter by convoy |
| `--watch` | Watch for updates |

## Convoy Commands

### `brat convoy create`

Create a new convoy.

```bash
brat convoy create --title <TITLE> --goal <GOAL>
```

| Option | Description |
|--------|-------------|
| `--title <text>` | Convoy title (required) |
| `--goal <text>` | Convoy goal (required) |
| `--mirror` | Create mirror convoy |
| `--repos <paths>` | Comma-separated repo paths (with `--mirror`) |
| `--workflow <name>` | Use workflow template |
| `--var <key=value>` | Workflow variable |

### `brat convoy list`

List all convoys.

```bash
brat convoy list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |
| `--all-repos` | Include all repos |

### `brat convoy show`

Show convoy details.

```bash
brat convoy show <CONVOY_ID> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |

### `brat convoy add-repo`

Add a repository to a convoy.

```bash
brat convoy add-repo <CONVOY_ID> --repo <PATH>
```

## Task Commands

### `brat task add`

Add a task.

```bash
brat task add [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--convoy <id>` | Parent convoy |
| `--solo` | Create single-task convoy |
| `--title <text>` | Task title (required) |
| `--paths <paths>` | Comma-separated paths |
| `--priority <P0\|P1\|P2>` | Task priority |
| `--repo <path>` | Target repo (multi-repo) |
| `--engine <name>` | Override engine |

### `brat task list`

List tasks.

```bash
brat task list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |
| `--all-repos` | Include all repos |
| `--label <label>` | Filter by label |

### `brat task show`

Show task details.

```bash
brat task show <TASK_ID> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |

### `brat task assign`

Assign a task to an actor.

```bash
brat task assign <TASK_ID> --assignee <ACTOR_ID>
```

### `brat task comment`

Add a comment to a task.

```bash
brat task comment <TASK_ID> --body <TEXT>
```

### `brat task close`

Close a task.

```bash
brat task close <TASK_ID> --reason <done|dropped>
```

## Mayor Commands

### `brat mayor start`

Start the Mayor session.

```bash
brat mayor start
```

### `brat mayor ask`

Send a prompt to the Mayor.

```bash
brat mayor ask "<PROMPT>"
```

### `brat mayor status`

Check Mayor session status.

```bash
brat mayor status
```

### `brat mayor stop`

Stop the Mayor session.

```bash
brat mayor stop
```

## Session Commands

### `brat session list`

List active sessions.

```bash
brat session list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |

### `brat session tail`

View session output.

```bash
brat session tail <SESSION_ID> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--lines <n>` | Number of lines (default: 50) |
| `--json` | JSON output |

### `brat session stop`

Stop a session.

```bash
brat session stop <SESSION_ID>
```

## Role Commands

### `brat witness run`

Run the Witness to spawn agents.

```bash
brat witness run [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--once` | Run once and exit |

### `brat refinery run`

Run the Refinery to process merge queue.

```bash
brat refinery run [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--once` | Run once and exit |

### `brat deacon run`

Run the Deacon for cleanup.

```bash
brat deacon run [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--once` | Run once and exit |

## Swarm Commands

### `brat swarm start`

Start multiple agents for a convoy.

```bash
brat swarm start --n <COUNT> --convoy <ID>
```

| Option | Description |
|--------|-------------|
| `--n <count>` | Number of agents |
| `--convoy <id>` | Target convoy |

### `brat swarm stop`

Stop agents for a convoy.

```bash
brat swarm stop --convoy <ID>
```

## Lock Commands

### `brat lock status`

View lock status.

```bash
brat lock status [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |

### `brat lock acquire`

Acquire a lock.

```bash
brat lock acquire --resource <R> --ttl <DURATION>
```

| Option | Description |
|--------|-------------|
| `--resource <r>` | Resource identifier |
| `--ttl <duration>` | Lock duration (e.g., "15m") |

### `brat lock renew`

Renew a lock.

```bash
brat lock renew --resource <R> --ttl <DURATION>
```

### `brat lock release`

Release a lock.

```bash
brat lock release --resource <R>
```

| Option | Description |
|--------|-------------|
| `--force` | Force release |

## Daemon Commands

### `brat daemon start`

Start the daemon.

```bash
brat daemon start [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--port <port>` | HTTP port (default: 3000) |
| `--idle-timeout <secs>` | Idle shutdown timeout |
| `--foreground` | Run in foreground |

### `brat daemon stop`

Stop the daemon.

```bash
brat daemon stop
```

### `brat daemon status`

Check daemon status.

```bash
brat daemon status [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--json` | JSON output |

### `brat daemon restart`

Restart the daemon.

```bash
brat daemon restart [OPTIONS]
```

### `brat daemon logs`

View daemon logs.

```bash
brat daemon logs [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-n <lines>` | Number of lines |

## Utility Commands

### `brat doctor`

Health check and repair.

```bash
brat doctor [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--check` | Read-only health check |
| `--rebuild` | Rebuild state |
| `--json` | JSON output |

### `brat sync`

Sync with remote.

```bash
brat sync [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--pull` | Pull from remote |
| `--push` | Push to remote |

### `brat export`

Export data.

```bash
brat export --format <md|json>
```

### `brat config validate`

Validate configuration.

```bash
brat config validate
```

### `brat feed`

View event feed.

```bash
brat feed [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--once` | Run once |
| `--follow` | Follow updates |
| `--timeout <ms>` | Timeout |

## Examples

### Full Workflow

```bash
# Initialize
brat init

# Create convoy
brat convoy create --title "Bug fixes" --goal "Fix P0 bugs"

# Add tasks
brat task add --convoy <id> --title "Fix crash" --paths src/app.rs

# Run agents
brat witness run --once

# Monitor
brat status --watch

# Merge
brat refinery run --once
```

### Using the Mayor

```bash
# Start Mayor
brat mayor start

# Analyze and create work
brat mayor ask "Analyze src/ and create tasks for bugs"

# View results
brat status

# Run agents
brat witness run --once
```

### CI/CD Integration

```bash
# Run without daemon
brat --no-daemon witness run --once
brat --no-daemon refinery run --once

# Check status
brat --no-daemon status --json
```
