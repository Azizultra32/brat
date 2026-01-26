# Daemons

Brat uses two optional daemons: `grited` (Grite substrate) and `bratd` (Brat harness).

## Grite Daemon (grited)

The Grite daemon is optional and exists only to improve performance. **Correctness never depends on it.**

### Responsibilities

- Maintain a warm materialized view for fast reads
- Handle concurrent CLI requests efficiently
- Refresh daemon lock heartbeat

### Non-Responsibilities

- Never rewrites refs or force-pushes
- Never writes to the working tree
- **No background sync** (sync is always explicit via `grite sync`)

### Auto-Spawn

Grite CLI auto-spawns `grited` on first command:

- Default idle timeout: 5 minutes
- Use `--no-daemon` to force local execution
- Use `grite daemon start --idle-timeout <secs>` for custom timeout

See [Grite daemon documentation](https://github.com/neul-labs/grite/blob/main/docs/daemon.md) for details.

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
|                 Grite CLI (grite)                           |
+----------------------------------------------------------+
|              Grite Daemon (grited) [optional]               |
|         Warm cache | Concurrent access | IPC              |
+----------------------------------------------------------+
|                    Git Repository                         |
|         refs/grite/wal | refs/grite/locks | sled           |
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
| grited crashes | Lock expires, CLI takes over automatically |

---

## Interaction with Grite

- Bratd uses Grite as the source of truth for all state
- All convoy/task/session state stored in Grite issues and comments
- If `grited` is running, bratd benefits from its warm cache
- Bratd works without `grited` by using Grite CLI directly
