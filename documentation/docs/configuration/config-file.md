# Config File Reference

Complete reference for `.brat/config.toml`.

## Example Configuration

```toml
# .brat/config.toml

[roles]
mayor_enabled = true
witness_enabled = true
refinery_enabled = true
deacon_enabled = true

[daemon]
port = 3000
idle_timeout_secs = 900

[swarm]
max_polecats = 6
worktree_root = ".grit/worktrees"
engine = "claude"

[engine]
default = "claude"
spawn_timeout_ms = 60000
send_timeout_ms = 5000
tail_timeout_ms = 10000
stop_timeout_ms = 10000
health_timeout_ms = 5000
spawn_retry = 1

[refinery]
max_parallel_merges = 2
rebase_strategy = "rebase"
required_checks = ["tests"]
merge_retry_limit = 2

[locks]
policy = "warn"

[interventions]
heartbeat_interval_ms = 30000
stale_session_ms = 300000
blocked_task_ms = 86400000

[logs]
retention_days = 7
```

## Section Reference

### `[roles]`

Enable or disable harness roles.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `mayor_enabled` | bool | `true` | Enable Mayor (AI orchestrator) |
| `witness_enabled` | bool | `true` | Enable Witness (session spawner) |
| `refinery_enabled` | bool | `true` | Enable Refinery (merge queue) |
| `deacon_enabled` | bool | `true` | Enable Deacon (janitor) |

```toml
[roles]
mayor_enabled = true
witness_enabled = true
refinery_enabled = true
deacon_enabled = true
```

### `[daemon]`

HTTP API daemon settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `port` | int | `3000` | HTTP API port |
| `idle_timeout_secs` | int | `900` | Shutdown after idle (0 = never) |

```toml
[daemon]
port = 3000
idle_timeout_secs = 900  # 15 minutes
```

### `[swarm]`

Agent session management.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_polecats` | int | `6` | Max concurrent agent sessions |
| `worktree_root` | string | `".grit/worktrees"` | Directory for worktrees |
| `engine` | string | `"claude"` | Default AI engine |

```toml
[swarm]
max_polecats = 6
worktree_root = ".grit/worktrees"
engine = "claude"
```

### `[engine]`

AI engine timeouts and settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default` | string | `"claude"` | Default engine |
| `spawn_timeout_ms` | int | `60000` | Spawn timeout (ms) |
| `send_timeout_ms` | int | `5000` | Send timeout (ms) |
| `tail_timeout_ms` | int | `10000` | Tail timeout (ms) |
| `stop_timeout_ms` | int | `10000` | Stop timeout (ms) |
| `health_timeout_ms` | int | `5000` | Health check timeout (ms) |
| `spawn_retry` | int | `1` | Number of spawn retries |

Supported engines: `claude`, `aider`, `opencode`, `codex`, `continue`, `gemini`, `copilot`

```toml
[engine]
default = "claude"
spawn_timeout_ms = 60000
spawn_retry = 2
```

### `[refinery]`

Merge queue settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_parallel_merges` | int | `2` | Max parallel merge attempts |
| `rebase_strategy` | string | `"rebase"` | Strategy: `rebase`, `squash`, `merge` |
| `required_checks` | array | `["tests"]` | Required CI checks |
| `merge_retry_limit` | int | `2` | Max retry attempts |

```toml
[refinery]
max_parallel_merges = 2
rebase_strategy = "squash"
required_checks = ["ci", "lint"]
merge_retry_limit = 3
```

### `[locks]`

Lock enforcement policy.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `policy` | string | `"warn"` | Policy: `off`, `warn`, `require` |

**Policy values:**

- `off` - No lock checks
- `warn` - Warn on conflicts, continue
- `require` - Block on conflicts

```toml
[locks]
policy = "require"
```

### `[interventions]`

Thresholds for detecting problems.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `heartbeat_interval_ms` | int | `30000` | Expected heartbeat interval |
| `stale_session_ms` | int | `300000` | Mark stale after (5 min) |
| `blocked_task_ms` | int | `86400000` | Mark blocked after (24 hr) |

```toml
[interventions]
heartbeat_interval_ms = 30000
stale_session_ms = 600000  # 10 minutes
blocked_task_ms = 43200000  # 12 hours
```

### `[logs]`

Log management.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `retention_days` | int | `7` | Days to retain logs |

```toml
[logs]
retention_days = 14
```

## Environment Variables

Override config values:

| Variable | Overrides |
|----------|-----------|
| `BRAT_DAEMON_PORT` | `daemon.port` |
| `BRAT_DAEMON_IDLE_TIMEOUT` | `daemon.idle_timeout_secs` |
| `BRAT_NO_DAEMON` | Disables daemon auto-start |
| `BRAT_HOME` | Global config directory |

```bash
BRAT_DAEMON_PORT=8080 brat daemon start
```

## Validation

Validate your configuration:

```bash
brat config validate
```

Returns non-zero on errors.

## Common Configurations

### Minimal

```toml
[engine]
default = "claude"
```

### Development

```toml
[daemon]
port = 3000
idle_timeout_secs = 0  # Never timeout

[swarm]
max_polecats = 2  # Limit concurrency

[engine]
default = "claude"
spawn_timeout_ms = 120000  # Longer timeouts
```

### Production

```toml
[daemon]
port = 3000
idle_timeout_secs = 3600  # 1 hour

[swarm]
max_polecats = 8

[refinery]
required_checks = ["ci", "lint", "security"]
merge_retry_limit = 3

[locks]
policy = "require"

[logs]
retention_days = 30
```

### CI/CD

```toml
[roles]
mayor_enabled = false  # No AI orchestration in CI
deacon_enabled = false

[daemon]
idle_timeout_secs = 300  # Quick shutdown

[swarm]
max_polecats = 4

[engine]
spawn_timeout_ms = 30000  # Faster failures
```
