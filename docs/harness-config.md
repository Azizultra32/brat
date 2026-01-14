# Harness configuration

This doc defines Brat-specific configuration. Grit configuration remains in `.git/grit/config.toml` and actor directories.

## Config locations

- Repo config: `.brat/config.toml` (gitignored)
- Optional global config: `$BRAT_HOME/config.toml`

## Core settings

```toml
[roles]
mayor_enabled = true
witness_enabled = true
refinery_enabled = true
deacon_enabled = true

[bratd]
enabled = true
start_gritd = false

[swarm]
max_polecats = 6
worktree_root = ".grit/worktrees"
engine = "codex"

[engine]
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
policy = "warn" # off|warn|require

[tmux]
session = "brat"
windows = ["mayor", "witness", "refinery", "deacon", "sessions"]

[repos]
roots = ["/path/to/repo-a", "/path/to/repo-b"]

[logs]
retention_days = 7

[interventions]
heartbeat_interval_ms = 30000
stale_session_ms = 300000
blocked_task_ms = 86400000
```

## Validation rules

- Unknown keys are rejected with a clear error.
- Missing required keys fall back to defaults.
- Invalid enum values (for example, lock policy) are rejected.
- `brat config validate` reports errors and exits non-zero.

## Relationship to Grit

- Brat never writes to `.git/grit/` directly.
- All task state flows through Grit issues, comments, labels, and locks.
