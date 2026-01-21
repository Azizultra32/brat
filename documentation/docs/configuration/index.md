# Configuration

Brat is configured through a TOML file and environment variables.

## Configuration Files

| Location | Purpose |
|----------|---------|
| `.brat/config.toml` | Repository-specific config |
| `$BRAT_HOME/config.toml` | Global config (optional) |

Repository config takes precedence over global config.

## Quick Start

Create `.brat/config.toml`:

```toml
[engine]
default = "claude"

[daemon]
port = 3000
idle_timeout_secs = 900

[swarm]
max_polecats = 6
```

Validate your configuration:

```bash
brat config validate
```

## Configuration Sections

<div class="grid cards" markdown>

-   :material-file-cog:{ .lg .middle } **Config File Reference**

    ---

    Complete reference for all configuration options.

    [:octicons-arrow-right-24: Reference](config-file.md)

-   :material-server:{ .lg .middle } **Daemon Configuration**

    ---

    Configure the HTTP API daemon (bratd).

    [:octicons-arrow-right-24: Daemon](daemon.md)

</div>

## Environment Variables

Override config file settings with environment variables:

| Variable | Description |
|----------|-------------|
| `BRAT_DAEMON_PORT` | Override daemon port |
| `BRAT_DAEMON_IDLE_TIMEOUT` | Override idle timeout (seconds) |
| `BRAT_NO_DAEMON` | Disable daemon auto-start |
| `BRAT_HOME` | Global config directory |

## Validation

Brat validates configuration on startup:

- Unknown keys are rejected
- Invalid values show clear errors
- Missing keys use defaults

Run validation manually:

```bash
brat config validate
```

## Relationship to Grit

| Config | Location | Purpose |
|--------|----------|---------|
| **Grit** | `.git/grit/config.toml` | Actor defaults, substrate settings |
| **Brat** | `.brat/config.toml` | Engine, daemon, workflow settings |

Key differences:

- Brat config is typically gitignored
- Grit config may be committed
- Both are TOML format
