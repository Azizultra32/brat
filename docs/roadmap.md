# Roadmap

## Status Overview

Grite (the substrate) is implemented in a separate repository and provides the foundation. This roadmap tracks Brat (the harness) development.

| Component | Status | Location |
|-----------|--------|----------|
| Grite substrate | **Complete** | `/home/dipankar/Code/grite` |
| Brat harness | **Milestone 5 complete** | This repo |

---

## Grite Milestones (Complete)

These milestones are implemented in the Grite repository.

### Milestone 1: Grite core (done)

- [x] Event model + hashing
- [x] sled projections
- [x] CLI: init, actor management, create/list/show/update/comment/close
- [x] Export to markdown/json (with schema)
- [x] Tests: deterministic rebuild
- [x] `grite db stats` output schema

### Milestone 2: Git WAL + sync (done)

- [x] WAL commit writer/reader
- [x] Push/pull `refs/grite/*`
- [x] Handle remote-advanced push (fast-forward rebase)
- [x] Snapshot support
- [x] Portable WAL encoding (CBOR)
- [x] Hash test vectors

### Milestone 3: Daemon + IPC (done)

- [x] Daemon discovery
- [x] `grite` routes all commands through the daemon if present for the selected `(repo, actor)`
- [x] Pub/sub notifications
- [x] Daemon ownership lock with lease/heartbeat
- [x] Multi-repo, multi-actor workers

### Milestone 4: Locks + team workflows (done)

- [x] Lease locks stored in refs
- [x] Lock GC
- [x] Lock policy enforcement (`off|warn|require`)
- [x] `grite lock status`

---

## Brat Milestones

### Milestone 5: Harness integration (Complete)

- [x] Map harness tasks to Grite issues and labels
- [x] State machine spec and implementation (see `docs/state-machine.md`)
- [x] Session reconciliation on startup and crash recovery
- [x] Observability contract (exit code, reason, last logs)
- [x] Non-blocking UX contract for harness commands
- [x] Witness/Refinery workflows post updates as Grite comments
- [x] Worktree manager for polecats
- [x] Engine integration for session lifecycle
- [x] Merge queue policy + retry rules
- [x] Lock discipline (`path:` and `repo:` usage)
- [x] Control room UX for health, sessions, queue, and locks
- [x] End-to-end integration tests (convoy-like flow)

### Milestone 6: Hardening (Current Focus)

- [x] Stress tests (concurrent writers) - in Grite repo
- [ ] Corruption recovery
- [x] Security signing/verification design
- [ ] Security signing/verification implementation
- [x] DB maintenance thresholds and docs
