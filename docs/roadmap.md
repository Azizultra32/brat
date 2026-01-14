# Roadmap (initial)

## Milestone 1: Grit core

- Event model + hashing
- sled projections
- CLI: init, actor management, create/list/show/update/comment/close
- Export to markdown/json (with schema)
- Tests: deterministic rebuild
- `grit db stats` output schema

## Milestone 2: Git WAL + sync

- WAL commit writer/reader
- Push/pull `refs/grit/*`
- Handle remote-advanced push (fast-forward rebase)
- Snapshot support
- Portable WAL encoding (CBOR)
- Hash test vectors

## Milestone 3: Daemon + IPC

- Daemon discovery
- `grit` routes all commands through the daemon if present for the selected `(repo, actor)`
- Pub/sub notifications
- Daemon ownership lock with lease/heartbeat
- Multi-repo, multi-actor workers

## Milestone 4: Locks + team workflows

- Lease locks stored in refs
- Lock GC
- Lock policy enforcement (`off|warn|require`)
- `grit lock status`

## Milestone 5: Harness integration

- Map harness tasks to Grit issues and labels
- State machine spec and implementation (see `docs/state-machine.md`)
- Session reconciliation on startup and crash recovery
- Observability contract (exit code, reason, last logs)
- Non-blocking UX contract for harness commands
- Witness/Refinery workflows post updates as Grit comments
- Worktree manager for polecats
- Engine integration for session lifecycle
- Merge queue policy + retry rules
- Lock discipline (`path:` and `repo:` usage)
- Control room UX for health, sessions, queue, and locks
- End-to-end integration tests (convoy-like flow)

## Milestone 6: Hardening

- Stress tests (concurrent writers)
- Corruption recovery
- Security (signing and verification)
- DB maintenance thresholds and docs
