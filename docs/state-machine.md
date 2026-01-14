# Harness state machine

This document specifies the lifecycle state machines used by the Brat harness and how they are persisted in Grit.

## Goals

- Explicit lifecycle states for roles and sessions
- Idempotent transitions (safe to replay)
- Recoverable after crash or restart
- Observable in the control room via Grit queries

## Session lifecycle

### States

- `spawned`: session created, not yet ready
- `ready`: engine healthy, initial prompt delivered
- `running`: actively executing task work
- `handoff`: waiting for review or merge
- `exit`: session terminated (success or failure)

### Session types

- Polecat: ephemeral worker session managed by Witness
- Crew: user-owned persistent session (manual lifecycle control)

### Transitions

- `spawned -> ready`: engine health check passes
- `ready -> running`: first task action begins
- `running -> handoff`: task ready for review or merge
- `running -> exit`: failure, timeout, or user stop
- `handoff -> exit`: task closed or session stopped

### Persistence in Grit

- Each transition is recorded as a Grit comment on the task issue
- Labels are updated to reflect the current state
- Exit transition includes exit code, reason, last output hash or snippet

## Role lifecycle

### States

- `idle`: not actively coordinating
- `active`: role is executing normal duties
- `degraded`: errors detected, partial capability
- `recovering`: reconciling state or restarting sessions

### Transitions

- `idle -> active`: role is invoked or scheduled
- `active -> degraded`: failed health check or missing resources
- `degraded -> recovering`: reconciliation begins
- `recovering -> active`: state is consistent again

### Persistence in Grit

- Role state transitions are recorded in a dedicated issue or log thread
- Health summaries are posted at bounded intervals

## Reconciliation rules

On harness startup:

1. List active engine sessions for each role
2. Read expected sessions from Grit comments/labels
3. Reconcile:
   - Adopt orphaned sessions and post a recovery note
   - Mark missing sessions as `exit` with best-known info
   - Re-emit a `ready` comment if a session is running but missing state

Reconciliation should be safe to run multiple times.

## Label conventions (recommended)

- Session state: `session:spawned|ready|running|handoff|exit`
- Session type: `session:polecat|crew`
- Role state: `role:idle|active|degraded|recovering`
- Health: `health:ok|warn|fail`
