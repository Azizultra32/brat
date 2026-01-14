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

[swarm]
max_polecats = 6
worktree_root = ".grit/worktrees"
engine = "codex"

[refinery]
max_parallel_merges = 2
rebase_strategy = "rebase"
required_checks = ["tests"]

[locks]
policy = "warn" # off|warn|require

[tmux]
session = "brat"
windows = ["mayor", "witness", "refinery", "deacon", "sessions"]
```

## Validation rules

- Unknown keys are rejected with a clear error.
- Missing required keys fall back to defaults.
- Invalid enum values (for example, lock policy) are rejected.

## Relationship to Grit

- Brat never writes to `.git/grit/` directly.
- All task state flows through Grit issues, comments, labels, and locks.
