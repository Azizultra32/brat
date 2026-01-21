# Troubleshooting

Common issues and how to fix them.

## Intervention Points

Brat surfaces problems via `brat status`. When intervention is needed, you'll see:

```
Interventions needed:
- stuck_session: s-20250121-abc1 missed heartbeat for 5m
  Actions:
    brat session tail s-20250121-abc1 --lines 200
    brat session stop s-20250121-abc1
    brat witness run --once
```

## Common Issues

### Stuck Session

**Symptoms:** Session shows no heartbeat for 5+ minutes.

**Diagnosis:**

```bash
brat session list --json
brat session tail <session-id> --lines 200
```

**Solutions:**

1. Wait a bit longer (API rate limits can cause delays)
2. Stop and retry:
   ```bash
   brat session stop <session-id>
   brat witness run --once
   ```
3. Check engine health and API keys

### Blocked Task

**Symptoms:** Task stuck in `status:blocked`.

**Diagnosis:**

```bash
brat task show <task-id>
```

**Solutions:**

1. Add missing context:
   ```bash
   brat task comment <task-id> --body "Additional information..."
   ```
2. Reassign to a different agent:
   ```bash
   brat task assign <task-id> --assignee <actor-id>
   ```
3. Requeue the task (stop current session first)

### Merge Failed

**Symptoms:** Task shows `merge:failed`.

**Diagnosis:**

```bash
brat task list --label merge:failed --json
```

**Solutions:**

1. Retry the merge:
   ```bash
   brat refinery run --once
   ```
2. Check for merge conflicts manually
3. Fix conflicts in the task branch

### Lock Conflict

**Symptoms:** Operations blocked by existing locks.

**Diagnosis:**

```bash
brat lock status --json
```

**Solutions:**

1. Wait for the lock holder to finish
2. Coordinate with the lock holder
3. Force release (if safe):
   ```bash
   brat lock release --resource "path:<path>" --force
   ```

### Config Error

**Symptoms:** Commands fail with config validation errors.

**Diagnosis:**

```bash
brat config validate
```

**Solutions:**

1. Check `.brat/config.toml` for syntax errors
2. Remove unknown keys
3. Fix invalid values

### Daemon Down

**Symptoms:** Commands fail to connect to daemon.

**Diagnosis:**

```bash
brat daemon status
```

**Solutions:**

1. Start the daemon:
   ```bash
   brat daemon start
   ```
2. Use standalone mode:
   ```bash
   brat --no-daemon status
   ```
3. Check daemon logs:
   ```bash
   brat daemon logs
   ```

### Projection Drift

**Symptoms:** Inconsistent state, stale data.

**Diagnosis:**

```bash
brat doctor --check
```

**Solutions:**

```bash
brat doctor --rebuild
```

## Health Checks

### Quick Check

```bash
brat doctor --check
```

Reports issues with:

- Grit WAL consistency
- Lock state
- Session health
- Configuration

### Full Rebuild

```bash
brat doctor --rebuild
```

Rebuilds all local projections from the WAL.

## Common Error Messages

### "No such convoy"

The convoy ID doesn't exist or was deleted.

```bash
brat convoy list  # See available convoys
```

### "Engine spawn timeout"

The AI engine took too long to start.

Solutions:

- Check API credentials
- Increase timeout in config:
  ```toml
  [engine]
  spawn_timeout_ms = 120000
  ```

### "Lock acquisition failed"

Another process holds a conflicting lock.

```bash
brat lock status  # See who holds the lock
```

### "WAL append failed"

Failed to write to the Grit event log.

Solutions:

- Check disk space
- Check Git permissions
- Run `grit doctor --fix`

## Getting Logs

### Daemon Logs

```bash
brat daemon logs
brat daemon logs -n 200
```

### Session Logs

```bash
brat session tail <session-id>
brat session tail <session-id> --lines 500
```

### Verbose Output

```bash
brat --verbose status
RUST_LOG=debug brat status
```

## Reporting Issues

If you encounter a bug:

1. Gather diagnostic info:
   ```bash
   brat --version
   brat doctor --check --json > doctor.json
   brat status --json > status.json
   ```

2. Report at [github.com/neul-labs/brat/issues](https://github.com/neul-labs/brat/issues)

Include:

- What you were trying to do
- The error message
- Diagnostic output
- Steps to reproduce

## Thresholds

Default intervention thresholds (configurable in `.brat/config.toml`):

| Threshold | Default | Config Key |
|-----------|---------|------------|
| Heartbeat interval | 30s | `interventions.heartbeat_interval_ms` |
| Stale session | 5m | `interventions.stale_session_ms` |
| Blocked task escalation | 24h | `interventions.blocked_task_ms` |
| Merge retry limit | 2 | `refinery.merge_retry_limit` |

Adjust for your workflow:

```toml
[interventions]
stale_session_ms = 600000  # 10 minutes
```
