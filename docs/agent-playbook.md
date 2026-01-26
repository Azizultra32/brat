# Agent Playbook

This repository uses Brat as the primary CLI and Grite as the substrate for task and memory state.

## Non-interactive contract

- Use `--json` for all reads.
- Do not run interactive commands (no editor prompts).
- Do not force-push `refs/grite/*`.
- On inconsistencies, run `brat doctor --check --json` and follow the plan.

## Startup routine

Run at the beginning of each session:

- `brat sync --pull --json`
- `brat task list --label agent:todo --json`
- `brat task list --label priority:P0 --json`

Select exactly one issue at a time.

## Shared repo note

If multiple agents share the same `.git` directory, each agent must use a separate data directory. Set `GRIT_HOME`, `--data-dir`, or `--actor <id>` so the local DB is not shared between processes.

## Plan format

Before coding, post a plan comment:

```
Intended changes: <files/modules>
Tests: <commands>
Rollback: <strategy>
```

## Checkpoints

After each milestone, post a checkpoint comment:

- What changed
- Why
- Tests run

## Locks

Acquire a lock when editing shared or risky areas:

- `brat lock acquire --resource "path:<FILE>" --ttl 15m --json`
- `brat lock renew --resource "path:<FILE>" --ttl 15m --json`
- `brat lock release --resource "path:<FILE>" --json`

If a lock is unavailable, pick another issue or coordinate in comments.

## Finish

Before closing:

- Post verification notes (commands + expected output)
- `brat task close <ID> --reason done --json`
- `brat sync --push --json`
