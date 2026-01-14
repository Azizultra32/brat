# Brat (Grit-backed)

Brat is an autonomous multi-agent coding harness. It uses Grit as the substrate for tasks and memory: an append-only event log in git refs plus a local materialized view.

This repository contains the design, data model, and implementation roadmap for the harness and its Grit integration.

## Why

- Keep state local, auditable, and diffable in git.
- Avoid worktree conflicts and tracked-file churn.
- Make merges deterministic and non-destructive.
- Require no daemon for correctness; `gritd` is optional and `bratd` runs by default for UX but is not required for correctness.

## Core design (one screen)

- Canonical task/memory state lives in an append-only WAL stored in `refs/grit/wal`.
- Local state is a deterministic materialized view in `.git/grit/actors/<actor_id>/sled/`.
- The harness (roles, swarm, tmux control room) reads/writes Grit issues, comments, and labels.
- Sync is `git fetch/push refs/grit/*` with monotonic fast-forward only.
- Conflicts are resolved by event union + deterministic projection rules.

## Repository layout (planned)

- `libgrit-core`: event types, hashing, projections, sled store
- `libgrit-git`: WAL commits, ref sync, snapshots
- `libgrit-ipc`: rkyv schemas + async-nng IPC
- `grit`: CLI
- `gritd`: optional daemon
- `brat`: harness CLI (roles, swarm, control room)
- `bratd`: harness daemon (role supervisor + tmux control room)

## Docs

Substrate (Grit):
- `docs/architecture.md`
- `docs/actors.md`
- `docs/data-model.md`
- `docs/hash-vectors.md`
- `docs/git-wal.md`
- `docs/cli.md`
- `docs/daemon.md`
- `docs/export-format.md`
- `docs/agent-playbook.md`
- `docs/locking.md`
- `docs/operations.md`
- `docs/grit-mapping.md`

Harness (Brat):
- `docs/roles.md`
- `docs/worktrees.md`
- `docs/engine.md`
- `docs/convoy-task-schema.md`
- `docs/session-event-schema.md`
- `docs/label-glossary.md`
- `docs/canonical-spec.md`
- `docs/brat-cli.md`
- `docs/usability.md`
- `docs/multi-repo.md`
- `docs/tmux-bootstrap.sh`
- `docs/bratd.md`
- `docs/harness-config.md`
- `docs/gastown-gap-closure.md`
- `docs/state-machine.md`
- `docs/why.md`
- `docs/naming.md`
- `docs/acceptance-tests.md`
- `docs/roadmap.md`

## Status

Design-first. The docs define the target architecture and implementation milestones.
