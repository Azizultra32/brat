# Architecture

## Overview

Brat is a multi-agent harness backed by Grite for task and memory storage. The harness provides roles, swarming, and orchestration UX. Grite provides the append-only ledger, deterministic projections, and an optional daemon (`grited`); Brat runs `bratd` by default for UX.

## Layers

1. **Harness layer (Brat)**
   - Roles: Mayor, Witness, Refinery, Deacon
   - Swarm orchestration and control room UX
   - Uses Grite issues, comments, labels, and locks for coordination

2. **Grite substrate (source of truth)**
   - Append-only events in `refs/grite/wal`
   - Local materialized view in `.git/grite/actors/<actor_id>/sled/`
   - Deterministic projections from the WAL, values encoded with `rkyv`

3. **Grite daemon (optional, performance only)**
   - Background fetch/push
   - Warm cache and pub/sub notifications

Correctness never depends on `grited`; the CLI can always rebuild state from the WAL. `bratd` runs by default for UX but is not required for correctness.

## Components

- `libgrite-core`: event types, hashing, projections, sled store
- `libgrite-git`: WAL commit read/write, snapshot handling, ref sync
- `libgrite-ipc`: shared IPC message schema (rkyv)
- `grite`: CLI frontend
- `grited`: daemon (optional)
- `brat`: harness CLI (roles, swarm, control room)
- `bratd`: harness daemon (role supervisor + tmux control room)

## Data flow

1. The harness creates or updates Grite issues and comments.
2. Events are appended to the WAL ref as a new git commit.
3. The local materialized view is updated from new WAL events.
4. `grite sync` pushes/pulls refs; the harness observes updates via the view.

## Storage footprint

Local state is scoped per actor. Each agent gets its own data directory to avoid multi-process writes to the same DB.

- `.git/grite/actors/<actor_id>/sled/`: local DB (per actor)
- `.git/grite/actors/<actor_id>/config.toml`: local config and actor identity
- `.git/grite/config.toml`: repo-level defaults (for example, default actor)
- `.grite/`: optional export output (gitignored)
- `refs/grite/wal`: append-only event log
- `refs/grite/snapshots/*`: optional, monotonic snapshots
- `refs/grite/locks/*`: optional lease locks
