# Managing Convoys

Convoys are groups of related tasks. This guide covers convoy operations.

## Creating Convoys

### Basic Creation

```bash
brat convoy create --title "Bug fixes" --goal "Fix critical bugs in v2.0"
```

Returns a convoy ID like `convoy-20250121-abc1`.

### With the Mayor

Let the Mayor create convoys based on analysis:

```bash
brat mayor start
brat mayor ask "Analyze src/ and create a convoy for the bugs you find"
```

### Mirror Convoy (Multi-Repo)

Create a convoy that spans multiple repositories:

```bash
brat convoy create --mirror --repos /path/to/repo1,/path/to/repo2
```

## Listing Convoys

```bash
# List all convoys
brat convoy list

# JSON output
brat convoy list --json

# Across all repos (when using multi-repo)
brat convoy list --all-repos
```

## Viewing Convoy Details

```bash
brat convoy show <convoy-id>

# JSON output
brat convoy show <convoy-id> --json
```

Shows:

- Convoy title and goal
- Status (active, paused, complete, failed)
- Task counts by status
- Creation date

## Convoy Status

| Status | Meaning |
|--------|---------|
| `active` | Convoy is accepting work |
| `paused` | Work paused temporarily |
| `complete` | All tasks finished |
| `failed` | Convoy aborted or critical failure |

### Checking Status

```bash
brat status
```

Shows all convoys and their task counts.

## Adding Tasks to a Convoy

```bash
brat task add \
  --convoy <convoy-id> \
  --title "Fix null pointer" \
  --paths src/service.rs
```

See [Managing Tasks](managing-tasks.md) for details.

## Multi-Repository Convoys

Add a repository to an existing convoy:

```bash
brat convoy add-repo <convoy-id> --repo /path/to/other-repo
```

Add tasks targeting specific repos:

```bash
brat task add \
  --convoy <convoy-id> \
  --repo /path/to/other-repo \
  --title "Update shared library"
```

## Solo Tasks

For one-off tasks without a convoy:

```bash
brat task add --solo --title "Quick fix" --paths src/app.rs
```

This creates a single-task convoy behind the scenes.

## Spawning Agents for a Convoy

Run the Witness to process tasks in a convoy:

```bash
# Process all queued tasks
brat witness run --once

# Limit to a specific convoy
brat swarm start --n 3 --convoy <convoy-id>
```

## Monitoring Progress

### CLI

```bash
# Watch status updates
brat status --watch

# View only convoy status
brat convoy show <convoy-id>
```

### Web Dashboard

Open `http://localhost:5173` and navigate to the Convoys tab.

## Completing a Convoy

A convoy automatically moves to `complete` when all tasks reach `merged` or `dropped` status.

To manually check completion:

```bash
brat convoy show <convoy-id> --json | jq '.task_summary'
```

## Example Workflow

```bash
# 1. Create a convoy
brat convoy create --title "Q1 Bug Fixes" --goal "Fix all P0/P1 bugs"
# Returns: convoy-20250121-xyz9

# 2. Add tasks
brat task add --convoy convoy-20250121-xyz9 \
  --title "Fix login timeout" --paths src/auth/

brat task add --convoy convoy-20250121-xyz9 \
  --title "Fix data corruption" --paths src/db/

# 3. Check status
brat status

# 4. Run agents
brat witness run --once

# 5. Monitor
brat status --watch

# 6. Merge completed work
brat refinery run --once
```

## Best Practices

### Keep Convoys Focused

- Group related work together
- Each convoy should have a clear goal
- Avoid mixing unrelated fixes

### Use Descriptive Titles

**Good:** "Authentication security improvements"
**Bad:** "Fixes"

### Set Clear Goals

The goal helps the Mayor and agents understand context:

```bash
brat convoy create \
  --title "API v2 migration" \
  --goal "Migrate all endpoints from v1 to v2 format while maintaining backward compatibility"
```

### Track Progress

Use `brat status --watch` or the web dashboard to monitor:

- How many tasks are queued vs. running
- Which tasks are blocked
- Merge status
