# Architecture

## Overview

Gems Town is a multi-agent harness backed by Grit for task and memory storage. The harness provides roles, swarming, and orchestration UX. Grit provides the append-only ledger, deterministic projections, and optional daemon.

## Layers

1. **Harness layer (Gems Town)**
   - Roles: Mayor, Witness, Refinery, Deacon
   - Swarm orchestration and control room UX
   - Uses Grit issues, comments, labels, and locks for coordination

2. **Grit substrate (source of truth)**
   - Append-only events in `refs/grit/wal`
   - Local materialized view in `.git/grit/actors/<actor_id>/sled/`
   - Deterministic projections from the WAL, values encoded with `rkyv`

3. **Optional daemon (performance only)**
   - Background fetch/push
   - Warm cache and pub/sub notifications

Correctness never depends on the daemon; the CLI can always rebuild state from the WAL.

## Components

- `libgrit-core`: event types, hashing, projections, sled store
- `libgrit-git`: WAL commit read/write, snapshot handling, ref sync
- `libgrit-ipc`: shared IPC message schema (rkyv)
- `grit`: CLI frontend
- `gritd`: daemon (optional)
- `gt`: harness CLI (roles, swarm, control room)

## Data flow

1. The harness creates or updates Grit issues and comments.
2. Events are appended to the WAL ref as a new git commit.
3. The local materialized view is updated from new WAL events.
4. `grit sync` pushes/pulls refs; the harness observes updates via the view.

## Storage footprint

Local state is scoped per actor. Each agent gets its own data directory to avoid multi-process writes to the same DB.

- `.git/grit/actors/<actor_id>/sled/`: local DB (per actor)
- `.git/grit/actors/<actor_id>/config.toml`: local config and actor identity
- `.git/grit/config.toml`: repo-level defaults (for example, default actor)
- `.grit/`: optional export output (gitignored)
- `refs/grit/wal`: append-only event log
- `refs/grit/snapshots/*`: optional, monotonic snapshots
- `refs/grit/locks/*`: optional lease locks
