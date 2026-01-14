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
- Witness/Refinery workflows post updates as Grit comments
- Worktree manager for polecats
- Engine integration for session lifecycle

## Milestone 6: Hardening

- Stress tests (concurrent writers)
- Corruption recovery
- Security (signing and verification)
- DB maintenance thresholds and docs
