# Daemons

Brat uses two optional daemons: `gritd` (Grit substrate) and `bratd` (Brat harness).

## Grit Daemon (gritd)

The Grit daemon is optional and exists only to improve performance. **Correctness never depends on it.**

### Responsibilities

- Maintain a warm materialized view for fast reads
- Handle concurrent CLI requests efficiently
- Refresh daemon lock heartbeat

### Non-Responsibilities

- Never rewrites refs or force-pushes
- Never writes to the working tree
- **No background sync** (sync is always explicit via `grit sync`)

### Auto-Spawn

Grit CLI auto-spawns `gritd` on first command:

- Default idle timeout: 5 minutes
- Use `--no-daemon` to force local execution
- Use `grit daemon start --idle-timeout <secs>` for custom timeout

See [Grit daemon documentation](https://github.com/neul-labs/grit/blob/main/docs/daemon.md) for details.

---

## Brat Daemon (bratd)

The Brat daemon provides the HTTP API for the web UI and coordinates multi-repo sessions.

### Responsibilities

- Serve REST API endpoints for convoys, tasks, sessions, and Mayor
- Track activity for idle shutdown (default: 15 minutes)
- Manage multi-repo state

### Commands

| Command | Description |
|---------|-------------|
| `brat daemon start` | Start in background |
| `brat daemon stop` | Stop gracefully |
| `brat daemon status` | Check if running |
| `brat daemon restart` | Restart with new settings |
| `brat daemon logs` | View log output |

### Options

```bash
brat daemon start [OPTIONS]

Options:
  -p, --port <PORT>           Port to listen on [default: 3000]
      --idle-timeout <SECS>   Idle timeout in seconds (0 = no timeout) [default: 900]
      --foreground            Run in foreground (don't daemonize)
```

### Auto-Start

Bratd auto-starts when you run commands that need it:

- `brat status`
- `brat convoy ...`
- `brat task ...`
- `brat session ...`
- `brat mayor ...`
- `brat witness ...`
- `brat refinery ...`

Use `--no-daemon` to disable auto-start for scripting:

```bash
brat --no-daemon status
```

### State Files

The daemon stores state in `~/.brat/`:

| File | Purpose |
|------|---------|
| `bratd.pid` | Process ID of running daemon |
| `bratd.log` | Daemon log output |

---

## Relationship

```
+----------------------------------------------------------+
|                    Brat CLI (brat)                        |
+----------------------------------------------------------+
|                 Brat Daemon (bratd)                       |
|   HTTP API | Multi-repo | Session tracking | Mayor        |
+----------------------------------------------------------+
|                 Grit CLI (grit)                           |
+----------------------------------------------------------+
|              Grit Daemon (gritd) [optional]               |
|         Warm cache | Concurrent access | IPC              |
+----------------------------------------------------------+
|                    Git Repository                         |
|         refs/grit/wal | refs/grit/locks | sled           |
+----------------------------------------------------------+
```

Both daemons are optional. All commands work without them (standalone mode).

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `BRAT_DAEMON_PORT` | Override daemon port | 3000 |
| `BRAT_DAEMON_IDLE_TIMEOUT` | Override idle timeout (seconds) | 900 |
| `BRAT_NO_DAEMON` | Disable auto-start globally | false |

---

## Failure Behavior

| Failure | Recovery |
|---------|----------|
| bratd can't start | CLI commands still work (standalone mode) |
| Auto-start fails | Warning shown, command continues |
| bratd crashes | Restart with `brat daemon start` |
| gritd crashes | Lock expires, CLI takes over automatically |

---

## Interaction with Grit

- Bratd uses Grit as the source of truth for all state
- All convoy/task/session state stored in Grit issues and comments
- If `gritd` is running, bratd benefits from its warm cache
- Bratd works without `gritd` by using Grit CLI directly
