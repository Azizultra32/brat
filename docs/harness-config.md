# Harness Configuration

This doc defines Brat-specific configuration. Grite configuration remains in `.git/grite/config.toml` and actor directories.

## Config Locations

| Location | Purpose |
|----------|---------|
| `.brat/config.toml` | Repo-specific config (gitignored) |
| `$BRAT_HOME/config.toml` | Optional global config |

---

## Example Config

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
worktree_root = ".gritee/worktrees"
engine = "codex"
engine_args = []

[engine]
default = "codex"
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
target_branch = "auto"

[locks]
policy = "warn"

[interventions]
heartbeat_interval_ms = 30000
stale_session_ms = 300000
blocked_task_ms = 86400000

[logs]
retention_days = 7
```

---

## Sections

### `[roles]`

Enable or disable harness roles.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `mayor_enabled` | bool | `true` | Enable Mayor (AI orchestrator) |
| `witness_enabled` | bool | `true` | Enable Witness (session spawner) |
| `refinery_enabled` | bool | `true` | Enable Refinery (merge queue) |
| `deacon_enabled` | bool | `true` | Enable Deacon (background janitor) |

### `[daemon]`

Bratd HTTP API server settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `port` | int | `3000` | HTTP API port |
| `idle_timeout_secs` | int | `900` | Idle shutdown timeout (0 = no timeout) |

### `[swarm]`

Polecat session management.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_polecats` | int | `6` | Max concurrent polecat sessions |
| `worktree_root` | string | `".gritee/worktrees"` | Directory for session worktrees |
| `engine` | string | `"codex"` | Default AI engine for polecats |
| `engine_args` | array | `[]` | Extra arguments passed to the configured witness engine; cleared when CLI overrides to a different engine |

### `[engine]`

AI engine timeouts and retry settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default` | string | `"codex"` | Default AI coding engine |
| `spawn_timeout_ms` | int | `60000` | Timeout for spawning engine |
| `send_timeout_ms` | int | `5000` | Timeout for sending prompts |
| `tail_timeout_ms` | int | `10000` | Timeout for reading output |
| `stop_timeout_ms` | int | `10000` | Timeout for stopping engine |
| `health_timeout_ms` | int | `5000` | Timeout for health checks |
| `spawn_retry` | int | `1` | Number of spawn retries |

Supported engines: `claude`, `claude-code`, `aider`, `opencode`, `codex`, `continue`, `gemini`, `copilot`, `shell`

For `engine = "shell"`:

- `engine_args = []` runs a built-in deterministic smoke worker.
- non-empty `engine_args` are treated as the argv passed to the platform shell executable.

### `[refinery]`

Merge queue settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `max_parallel_merges` | int | `2` | Max parallel merge attempts |
| `rebase_strategy` | string | `"rebase"` | Strategy: `rebase`, `squash`, `merge` |
| `required_checks` | array | `["tests"]` | Required CI checks before merge |
| `merge_retry_limit` | int | `2` | Max retry attempts for conflicts |
| `target_branch` | string | `"auto"` | Integration branch or `auto` to resolve from local `origin/HEAD`, then `git remote show origin`; errors if unresolved |

### `[locks]`

Brat-level lock enforcement (independent of Grite's lock policy).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `policy` | string | `"warn"` | Lock policy: `off`, `warn`, `require` |

**Policy values:**

- `off` - No lock checks
- `warn` - Warn on conflicts, but continue
- `require` - Block operations if conflicting lock exists

### `[interventions]`

Thresholds for detecting problems and triggering interventions.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `heartbeat_interval_ms` | int | `30000` | Expected heartbeat interval |
| `stale_session_ms` | int | `300000` | Mark session stale after this |
| `blocked_task_ms` | int | `86400000` | Mark task blocked after this |

### `[logs]`

Log management.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `retention_days` | int | `7` | Days to retain log files |

---

## Environment Variables

Environment variables override config file settings:

| Variable | Description | Default |
|----------|-------------|---------|
| `BRAT_DAEMON_PORT` | Override daemon port | 3000 |
| `BRAT_DAEMON_IDLE_TIMEOUT` | Override idle timeout (seconds) | 900 |
| `BRAT_NO_DAEMON` | Disable daemon auto-start | false |
| `BRAT_HOME` | Global config directory | `~/.brat` |

---

## Validation

- Unknown keys are rejected with a clear error
- Missing required keys fall back to defaults
- Invalid enum values (e.g., lock policy) are rejected
- `brat config validate` reports errors and exits non-zero

---

## Relationship to Grite Config

| Config | Location | Purpose |
|--------|----------|---------|
| Grite | `.git/grite/config.toml` | Actor defaults, substrate lock policy |
| Brat | `.brat/config.toml` | Engine, daemon, witness settings |

Key differences:

- Brat never writes to `.git/grite/` directly
- All task state flows through Grite issues, comments, labels, and locks
- Brat-level lock policy is enforced in addition to Grite-level policy
- Brat config is gitignored; Grite config may be committed
