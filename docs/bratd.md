# Brat Daemon (bratd)

`bratd` is the HTTP API server for the brat harness. It provides REST endpoints for managing convoys, tasks, sessions, and the Mayor across multiple repositories. The daemon supports auto-start on CLI commands and idle shutdown for resource efficiency.

## Quick Start

```bash
# Start daemon in background (auto-starts on most commands anyway)
brat daemon start

# Check status
brat daemon status

# View logs
brat daemon logs -n 50

# Stop daemon
brat daemon stop
```

## CLI Commands

### `brat daemon start`

Start the daemon in background mode.

```bash
brat daemon start [OPTIONS]

Options:
  -p, --port <PORT>           Port to listen on [default: 3000]
      --idle-timeout <SECS>   Idle timeout in seconds (0 = no timeout) [default: 900]
      --foreground            Run in foreground (don't daemonize)
```

Examples:
```bash
# Start with defaults (port 3000, 15 min idle timeout)
brat daemon start

# Custom port and longer timeout
brat daemon start --port 8080 --idle-timeout 3600

# Run in foreground (for debugging)
brat daemon start --foreground

# Disable idle timeout (run forever)
brat daemon start --idle-timeout 0
```

### `brat daemon stop`

Stop the running daemon gracefully.

```bash
brat daemon stop
```

### `brat daemon status`

Show daemon status (running/stopped, PID, URL).

```bash
brat daemon status [--json]
```

Output:
```
Daemon is running
  PID:  12345
  URL:  http://127.0.0.1:3000
```

### `brat daemon restart`

Restart the daemon with new settings.

```bash
brat daemon restart [OPTIONS]

Options:
  -p, --port <PORT>           Port to listen on [default: 3000]
      --idle-timeout <SECS>   Idle timeout in seconds [default: 900]
```

### `brat daemon logs`

View daemon log output.

```bash
brat daemon logs [OPTIONS]

Options:
  -n, --lines <N>   Number of lines to show [default: 50]
```

## Standalone Binary (bratd)

The daemon is also available as a standalone binary:

```bash
bratd [OPTIONS]

Options:
      --host <HOST>           Host to bind to [default: 127.0.0.1]
  -p, --port <PORT>           Port to listen on [default: 3000]
      --idle-timeout <SECS>   Idle timeout in seconds (0 = no timeout) [default: 900]
      --cors-origin <ORIGIN>  CORS allowed origin (default: allow all)
```

## Auto-Start Behavior

The daemon automatically starts when you run commands that benefit from it:

- `brat status`
- `brat convoy ...`
- `brat task ...`
- `brat session ...`
- `brat mayor ...`
- `brat witness ...`
- `brat refinery ...`

To disable auto-start (e.g., for scripting):

```bash
brat --no-daemon status
```

## Idle Shutdown

By default, the daemon shuts down after 15 minutes (900 seconds) of inactivity. This conserves resources when not actively using brat.

- Activity is tracked on every API request
- The idle checker runs every 30 seconds
- Graceful shutdown on Ctrl+C or idle timeout

Configure via:
- `--idle-timeout <SECS>` flag (0 to disable)
- Environment: `BRAT_DAEMON_IDLE_TIMEOUT`

## State Files

The daemon stores state in `~/.brat/`:

| File | Purpose |
|------|---------|
| `bratd.pid` | Process ID of running daemon |
| `bratd.log` | Daemon log output |

## HTTP API Endpoints

Base URL: `http://127.0.0.1:3000/api/v1`

### Health
- `GET /health` - Daemon health check

### Repositories
- `GET /repos` - List registered repositories
- `POST /repos` - Register a repository
- `DELETE /repos/:repo_id` - Unregister a repository

### Status (per-repo)
- `GET /repos/:repo_id/status` - Repository status summary

### Convoys
- `GET /repos/:repo_id/convoys` - List convoys
- `POST /repos/:repo_id/convoys` - Create convoy
- `GET /repos/:repo_id/convoys/:id` - Get convoy details

### Tasks
- `GET /repos/:repo_id/tasks` - List tasks (with filters)
- `POST /repos/:repo_id/tasks` - Create task
- `GET /repos/:repo_id/tasks/:id` - Get task details
- `PATCH /repos/:repo_id/tasks/:id` - Update task status

### Sessions
- `GET /repos/:repo_id/sessions` - List sessions
- `GET /repos/:repo_id/sessions/:id` - Get session details
- `POST /repos/:repo_id/sessions/:id/stop` - Stop session
- `GET /repos/:repo_id/sessions/:id/logs` - Get session logs

### Mayor
- `GET /repos/:repo_id/mayor/status` - Mayor status
- `POST /repos/:repo_id/mayor/start` - Start Mayor
- `POST /repos/:repo_id/mayor/stop` - Stop Mayor
- `POST /repos/:repo_id/mayor/ask` - Send message to Mayor
- `GET /repos/:repo_id/mayor/history` - Get conversation history

## Multi-Repo Support

- One daemon manages multiple repositories
- Repositories are identified by base64-encoded paths
- Register repos via API: `POST /repos` with `{"path": "/path/to/repo"}`

## Interaction with Grit

- Daemon uses Grit as the source of truth
- All convoy/task/session state stored in Grit issues and comments
- If `gritd` is running, bratd benefits from its warm cache
- Daemon works without `gritd` by using Grit CLI directly

## Failure Behavior

- If daemon can't start, CLI commands still work (standalone mode)
- Auto-start failures show warnings but don't block commands
- Graceful shutdown on SIGTERM, force kill on SIGKILL

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `BRAT_DAEMON_PORT` | Override default port | 3000 |
| `BRAT_DAEMON_IDLE_TIMEOUT` | Override idle timeout (seconds) | 900 |
| `BRAT_NO_DAEMON` | Disable auto-start globally | false |
