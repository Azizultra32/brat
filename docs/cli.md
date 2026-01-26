# CLI

## Principles

- Non-interactive by default
- Structured output always available (`--json`)
- No `grited` required for correctness

## Grite command overview

- `grite init`
- `grite actor init [--label <name>]`
- `grite actor list [--json]`
- `grite actor show [<id>] [--json]`
- `grite actor current [--json]`
- `grite actor use <id>`
- `grite issue create --title ... --body ... --label ...`
- `grite issue update <id> [--title ...] [--body ...]`
- `grite issue list --state open --label bug --json`
- `grite issue show <id> --json`
- `grite issue comment <id> --body ...`
- `grite issue close <id> --reason done`
- `grite sync [--pull] [--push]`
- `grite doctor [--json] [--apply]`
- `grite rebuild`
- `grite db stats [--json]`
- `grite export --format md|json`
- `grite snapshot`
- `grite snapshot gc`
- `grite lock acquire --resource <R> --ttl 15m`
- `grite lock renew --resource <R> --ttl 15m`
- `grite lock release --resource <R>`
- `grite lock status [--json]`
- `grite lock gc`
- `grite daemon status [--json]`
- `grite daemon stop`

## JSON output

- `--json` is supported on all read commands
- `--quiet` suppresses human output for agents
- Errors are returned with structured details

## Data directory

- `GRIT_HOME` or `--data-dir` sets the local state root for this process
- Default is `.git/grite/actors/<actor_id>/`
- Each concurrent agent should use a distinct data dir
- If a daemon owns the selected data dir, the CLI routes all commands through it and does not open the DB directly

## Harness integration

Brat roles and automation use the Grite CLI (or libraries) to create and update issues, comments, labels, and locks. The harness never writes tracked files for metadata.
