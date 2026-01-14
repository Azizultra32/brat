# CLI

## Principles

- Non-interactive by default
- Structured output always available (`--json`)
- No `gritd` required for correctness

## Grit command overview

- `grit init`
- `grit actor init [--label <name>]`
- `grit actor list [--json]`
- `grit actor show [<id>] [--json]`
- `grit actor current [--json]`
- `grit actor use <id>`
- `grit issue create --title ... --body ... --label ...`
- `grit issue update <id> [--title ...] [--body ...]`
- `grit issue list --state open --label bug --json`
- `grit issue show <id> --json`
- `grit issue comment <id> --body ...`
- `grit issue close <id> --reason done`
- `grit sync [--pull] [--push]`
- `grit doctor [--json] [--apply]`
- `grit rebuild`
- `grit db stats [--json]`
- `grit export --format md|json`
- `grit snapshot`
- `grit snapshot gc`
- `grit lock acquire --resource <R> --ttl 15m`
- `grit lock renew --resource <R> --ttl 15m`
- `grit lock release --resource <R>`
- `grit lock status [--json]`
- `grit lock gc`
- `grit daemon status [--json]`
- `grit daemon stop`

## JSON output

- `--json` is supported on all read commands
- `--quiet` suppresses human output for agents
- Errors are returned with structured details

## Data directory

- `GRIT_HOME` or `--data-dir` sets the local state root for this process
- Default is `.git/grit/actors/<actor_id>/`
- Each concurrent agent should use a distinct data dir
- If a daemon owns the selected data dir, the CLI routes all commands through it and does not open the DB directly

## Harness integration

Brat roles and automation use the Grit CLI (or libraries) to create and update issues, comments, labels, and locks. The harness never writes tracked files for metadata.
