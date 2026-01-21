# How Brat Works

Brat is organized in layers, each with a clear responsibility.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                       Your Repository                        │
├──────────────────────────┬──────────────────────────────────┤
│  .brat/                  │  refs/grit/wal                   │
│  ├─ config.toml          │  └─ append-only event log        │
│  └─ workflows/           │                                  │
│     ├─ feature.yaml      │  .git/grit/actors/<id>/sled/     │
│     ├─ fix-bug.yaml      │  └─ local materialized view      │
│     └─ code-review.yaml  │                                  │
├──────────────────────────┴──────────────────────────────────┤
│                    Brat Harness Layer                        │
│  ┌────────┐ ┌─────────┐ ┌──────────┐ ┌────────┐            │
│  │ Mayor  │ │ Witness │ │ Refinery │ │ Deacon │            │
│  └────────┘ └─────────┘ └──────────┘ └────────┘            │
├─────────────────────────────────────────────────────────────┤
│                   Grit Substrate Layer                       │
│  Events • Issues • Labels • Comments • Locks • Sync         │
├─────────────────────────────────────────────────────────────┤
│                    AI Engine Adapters                        │
│  Claude │ Aider │ OpenCode │ Codex │ Continue │ Gemini      │
└─────────────────────────────────────────────────────────────┘
```

## Layers Explained

### 1. AI Engine Adapters

The bottom layer interfaces with AI coding tools. Each engine adapter:

- Spawns agent sessions
- Sends prompts and inputs
- Captures outputs
- Reports health status

Supported engines include Claude Code, Aider, Codex, OpenCode, Continue, and Gemini.

### 2. Grit Substrate

[Grit](https://github.com/neul-labs/grit) provides the persistence layer:

- **Write-Ahead Log (WAL)** - Append-only event storage in `refs/grit/wal`
- **Issues** - Track convoys and tasks
- **Labels** - Store status and metadata
- **Comments** - Record progress and outputs
- **Locks** - Coordinate resources
- **Sync** - Push/pull between repositories

The WAL ensures crash safety. All state can be rebuilt by replaying events.

### 3. Brat Harness Layer

The harness implements the orchestration logic:

| Role | Responsibility |
|------|----------------|
| **Mayor** | AI orchestrator - analyzes code, creates convoys/tasks |
| **Witness** | Spawns and monitors agent sessions |
| **Refinery** | Manages merge queue and integration |
| **Deacon** | Janitor - cleans locks, detects orphans, syncs state |

### 4. Your Repository

Your code, plus:

- `.brat/config.toml` - Configuration file
- `.brat/workflows/` - Reusable workflow templates

## Data Flow

```mermaid
sequenceDiagram
    participant User
    participant Mayor
    participant Grit
    participant Witness
    participant Engine
    participant Refinery

    User->>Mayor: Ask to analyze code
    Mayor->>Grit: Create convoy issue
    Mayor->>Grit: Create task issues

    Witness->>Grit: Query queued tasks
    Witness->>Engine: Spawn session
    Engine->>Engine: Work on task
    Engine-->>Witness: Complete
    Witness->>Grit: Update task status

    Refinery->>Grit: Query completed tasks
    Refinery->>Refinery: Apply merge policy
    Refinery->>Grit: Update merge status
```

## Storage Locations

| Path | Purpose |
|------|---------|
| `refs/grit/wal` | Append-only event log |
| `.git/grit/actors/<id>/sled/` | Local materialized view per actor |
| `.git/grit/config.toml` | Repo-level Grit config |
| `.brat/config.toml` | Brat configuration |
| `.brat/workflows/` | Workflow templates |

## The Daemon

Brat includes an optional daemon (`bratd`) that provides:

- HTTP API for the web UI
- Multi-session coordination
- Background role supervision
- Idle timeout and auto-shutdown

The daemon is not required for correctness. All CLI commands work standalone.

```bash
# Start the daemon
brat daemon start

# Commands auto-start the daemon by default
# Disable with --no-daemon
brat --no-daemon status
```

## Design Decisions

### Why Append-Only?

Traditional approaches use mutable state (databases, files). This causes:

- Silent failures when crashes corrupt state
- No audit trail of what happened
- Difficulty coordinating multiple writers

Brat's append-only log:

- Never loses events
- Enables deterministic replay
- Provides complete audit trail

### Why Not Use Git Branches?

Using git branches for coordination causes:

- Merge conflicts with metadata
- Dirty working trees
- Race conditions between processes

Brat stores all coordination state in Grit refs, keeping your working tree clean.

### Why Multiple Roles?

Separation of concerns:

- **Mayor** focuses on planning, not execution
- **Witness** focuses on agents, not merging
- **Refinery** focuses on integration, not spawning
- **Deacon** handles cleanup nobody else should worry about

Each role has a clear boundary and can be run independently.
