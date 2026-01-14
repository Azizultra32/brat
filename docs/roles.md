# Roles

Gems Town preserves the Gastown role topology. Roles are behavioral conventions that emit explicit actions via Grit issues, comments, labels, and locks.

## Mayor (control plane)

- Creates task issues in Grit
- Assigns tasks and updates labels/state
- Monitors status via Grit queries

## Witness (worker controller)

- Spawns agent sessions (polecats)
- Monitors heartbeats and progress
- Posts session updates as Grit comments or labels

## Refinery (integration controller)

- Consumes completed task outputs
- Manages merge queue policy
- Posts merge results as Grit updates (labels, comments, links)

## Deacon (janitor/reconciler)

- Expires or cleans up stale locks
- Detects orphan sessions
- Rebuilds projections if needed
- Syncs refs with remotes
