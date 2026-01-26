# Quickstart

Get up and running with Brat in 5 minutes.

## Initialize Your Repository

Navigate to your project and initialize both Grite and Brat:

```bash
cd your-project
grite init      # Initialize the Grite substrate
brat init      # Initialize the Brat harness
```

This creates:

- `refs/grite/wal` - The append-only event log
- `.brat/config.toml` - Brat configuration file

## Check Status

View the current state of your harness:

```bash
brat status
```

You'll see an empty status initially since there are no convoys or tasks yet.

## Start the Mayor

The Mayor is Brat's AI orchestrator. Start it:

```bash
brat mayor start
```

Ask the Mayor to analyze your code:

```bash
brat mayor ask "Analyze the codebase and identify any bugs or issues"
```

The Mayor will:

1. Scan your codebase
2. Identify issues
3. Create a convoy with tasks for each issue

## View Tasks

Check what the Mayor created:

```bash
brat status
```

You'll see convoys and tasks with their current status.

## Spawn Agents

Run the Witness to spawn AI agents that work on tasks:

```bash
brat witness run --once
```

This:

1. Picks up queued tasks
2. Spawns AI coding agents in isolated worktrees
3. Monitors their progress

## Monitor Progress

Watch the status in real-time:

```bash
brat status --watch
```

Or open the web dashboard:

```bash
# Start the daemon if not running
brat daemon start

# The UI is available at http://localhost:5173
```

## Next Steps

- [Your First Convoy](first-convoy.md) - Detailed tutorial
- [Using the Mayor](../guides/using-the-mayor.md) - Learn Mayor commands
- [Web Dashboard](../guides/web-dashboard.md) - Dashboard features
