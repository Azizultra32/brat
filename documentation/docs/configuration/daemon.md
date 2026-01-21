# Daemon Configuration

The Brat daemon (`bratd`) provides the HTTP API for the web UI and multi-session coordination.

## Overview

The daemon:

- Serves the HTTP API for the web dashboard
- Manages multiple concurrent sessions
- Supervises harness roles
- Auto-starts and auto-stops based on activity

## Starting the Daemon

### Auto-Start (Default)

Most commands auto-start the daemon when needed:

```bash
brat status  # Starts daemon if not running
```

Disable auto-start:

```bash
brat --no-daemon status
```

### Manual Start

```bash
# Start in background
brat daemon start

# Start with custom port
brat daemon start --port 8080

# Start with longer idle timeout
brat daemon start --idle-timeout 3600

# Start in foreground (for debugging)
brat daemon start --foreground
```

## Stopping the Daemon

```bash
brat daemon stop
```

Or set an idle timeout - the daemon stops automatically after inactivity.

## Checking Status

```bash
brat daemon status

# JSON output
brat daemon status --json
```

## Restarting

```bash
brat daemon restart

# With new settings
brat daemon restart --port 8080
```

## Configuration

### In Config File

```toml
# .brat/config.toml
[daemon]
port = 3000
idle_timeout_secs = 900  # 15 minutes
```

### Environment Variables

```bash
BRAT_DAEMON_PORT=8080 brat daemon start
BRAT_DAEMON_IDLE_TIMEOUT=3600 brat daemon start
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `port` | `3000` | HTTP API port |
| `idle_timeout_secs` | `900` | Shutdown after idle (0 = never) |

## Idle Timeout

The daemon shuts down after `idle_timeout_secs` of no activity:

- API requests reset the timer
- Active sessions prevent shutdown
- Set to `0` to disable timeout

```toml
[daemon]
idle_timeout_secs = 0  # Never timeout
```

## Standalone Binary

The daemon is also available as a standalone binary:

```bash
# Install
cargo install --path crates/brat --bin bratd

# Run
bratd --port 3000 --idle-timeout 900
```

## Logs

View daemon logs:

```bash
brat daemon logs

# Last N lines
brat daemon logs -n 200
```

Log location: `~/.brat/logs/bratd.log`

## API Endpoints

The daemon serves these endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/status` | GET | Harness status |
| `/api/convoys` | GET, POST | Convoy operations |
| `/api/convoys/:id` | GET | Convoy details |
| `/api/tasks` | GET, POST | Task operations |
| `/api/tasks/:id` | GET, PUT | Task details |
| `/api/sessions` | GET | Session list |
| `/api/sessions/:id` | GET, DELETE | Session details |
| `/api/mayor` | POST | Mayor commands |

## WebSocket

The daemon provides real-time updates via WebSocket:

```
ws://localhost:3000/ws
```

Events:

- Task status changes
- Session heartbeats
- Convoy updates

## Multi-Repository

One daemon can manage multiple repositories:

```bash
# Check which repos are registered
brat daemon status --json | jq '.repos'
```

Commands target the current directory by default. Use `--repo` to specify:

```bash
brat --repo /path/to/other-repo status
```

## Troubleshooting

### Daemon Won't Start

Check if port is in use:

```bash
lsof -i :3000
```

Use a different port:

```bash
brat daemon start --port 3001
```

### Can't Connect

Verify daemon is running:

```bash
brat daemon status
```

Check the logs:

```bash
brat daemon logs
```

### Daemon Keeps Stopping

Increase idle timeout:

```toml
[daemon]
idle_timeout_secs = 3600  # 1 hour
```

Or disable:

```toml
[daemon]
idle_timeout_secs = 0  # Never
```

## Running Without Daemon

All CLI commands work without the daemon:

```bash
brat --no-daemon status
brat --no-daemon witness run --once
```

The daemon is optional for correctness - it provides UX benefits but isn't required.

Use `--no-daemon` for:

- CI/CD pipelines
- Scripts
- Minimal resource usage
