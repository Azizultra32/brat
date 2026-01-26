# Your First Convoy

This tutorial walks you through creating your first convoy with AI agents working on real tasks.

## What You'll Build

By the end of this tutorial, you'll have:

- Created a convoy (a group of related tasks)
- Added tasks to the convoy
- Spawned AI agents to work on the tasks
- Monitored their progress

## Prerequisites

- Brat and Grite installed ([Installation](installation.md))
- An AI engine configured (e.g., Claude Code, Aider)
- A project repository to work on

## Step 1: Initialize the Repository

If you haven't already:

```bash
cd your-project
grite init
brat init
```

## Step 2: Create a Convoy

A convoy is a group of related tasks. Create one:

```bash
brat convoy create --title "Bug fixes" --goal "Fix critical bugs in the codebase"
```

This returns a convoy ID like `convoy-abc123`.

## Step 3: Add Tasks

Add tasks to your convoy:

```bash
# Add a task targeting specific files
brat task add \
  --convoy convoy-abc123 \
  --title "Fix null pointer in user service" \
  --paths src/services/user.rs

# Add another task
brat task add \
  --convoy convoy-abc123 \
  --title "Handle error case in login flow" \
  --paths src/auth/login.rs
```

## Step 4: View Status

Check what you've created:

```bash
brat status
```

You'll see output like:

```
Convoys:
  convoy-abc123 "Bug fixes" (2 tasks)

Tasks:
  task-def456 "Fix null pointer in user service" [queued]
  task-ghi789 "Handle error case in login flow" [queued]
```

## Step 5: Spawn Agents

Run the Witness to start agents working on tasks:

```bash
brat witness run --once
```

The Witness:

1. Creates isolated git worktrees for each task
2. Spawns your configured AI engine
3. Provides the task context to the agent

## Step 6: Monitor Progress

Watch tasks as agents work on them:

```bash
brat status --watch
```

View live logs from a session:

```bash
brat session list
brat session tail <session-id>
```

## Step 7: Review Results

When a task completes, the agent's changes are on a task branch. Review them:

```bash
git log --oneline task-def456
git diff main..task-def456
```

## Step 8: Merge with Refinery

The Refinery manages the merge queue:

```bash
brat refinery run --once
```

This applies your configured merge policy (rebase, squash, or merge).

## Using the Mayor (Alternative)

Instead of manually creating convoys and tasks, use the Mayor:

```bash
# Start the Mayor
brat mayor start

# Ask it to analyze and create tasks
brat mayor ask "Analyze src/ and create tasks for any bugs you find"

# The Mayor will create a convoy and tasks automatically
brat status
```

## Troubleshooting

### No tasks being picked up

Check that tasks are in `queued` status:

```bash
brat task list --json | jq '.[] | select(.status == "queued")'
```

### Agent not starting

Verify your engine is configured in `.brat/config.toml`:

```toml
[engine]
default = "claude"
```

### Session crashed

View session details:

```bash
brat session list --json
```

The Deacon cleans up crashed sessions automatically:

```bash
brat deacon run --once
```

## Next Steps

- [Managing Convoys](../guides/managing-convoys.md) - Advanced convoy operations
- [Workflow Templates](../guides/workflows.md) - Reusable workflow patterns
- [Configuration](../configuration/config-file.md) - Customize engine settings
