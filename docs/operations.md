# Operations

## Doctor

`brat doctor --check` performs read-only harness checks. It probes the Grite
projection with `grite --no-daemon issue list --json` and reads maintenance
signals with `grite --no-daemon db stats --json` so operators can see whether
Brat can recover without relying on `bratd` or `grited`.

`grite doctor` performs substrate-level checks and prints a remediation plan. It
never rewrites refs.

Checks include:

- WAL ref exists and is monotonic
- Local materialized view matches WAL head
- Actor identity is present
- Remote refs are reachable (optional)
- Locks are not stale (optional)

If `brat doctor --check --json` reports `gritee_projection_accessible` as a
warning or failure, use this non-destructive recovery ladder:

1. Wait briefly and rerun `brat --no-daemon doctor --check --json`.
2. Check daemon state with `brat daemon status --json` and
   `grite daemon status --json`.
3. Stop stale daemons with `brat daemon stop` or `grite daemon stop`.
4. Rerun `brat --no-daemon doctor --check --json`.
5. For local Grite projection repair, run `grite doctor --fix --json` or
   `grite rebuild`.
6. Rerun `brat status --json` and `grite sync --pull --json`.

Do not delete `.git/grite`, rewrite `refs/grite/*`, reset branches, or edit
tracked files as part of DB recovery.

If `brat doctor --check --json` reports `gritee_db_maintenance` as a warning,
follow the remediation on that check. It is advisory: the WAL remains canonical,
and the local projection can be rebuilt without changing tracked files.

For projection recovery, `grite doctor --fix` is expected to run safe local
repairs and never push refs:

- rebuild local DB
- fetch refs

If a future `grite doctor --fix` remediation would append canonical WAL commits,
it must report that separately from projection repair. If remote sync is needed,
the remediation plan explicitly lists `grite sync --pull` and/or
`grite sync --push`.

## Rebuild

`brat doctor --rebuild` reconciles Brat harness state: stale sessions, crashed
session labels, and abandoned worktrees. It does not rewrite Grite WAL refs and
does not repair the Grite projection itself.

`grite rebuild` discards the local sled view and replays:

1. Latest snapshot (if present)
2. WAL commits after the snapshot

Rebuilds also compact the local DB because they rewrite it from scratch.

## Local DB maintenance

The sled DB is a local projection cache. Prefer tool-mediated repair over manual
file deletion:

- `brat --no-daemon doctor --check --json` for harness-visible pass/warn/fail
  checks, including `gritee_projection_accessible` and `gritee_db_maintenance`
- `grite db stats --json` for size and last rebuild metadata
- `grite doctor --fix --json` for safe local repairs
- `grite rebuild` when the DB appears bloated or after crashes

Run Grite maintenance reads serially against a given worktree. The local sled
projection is single-writer; parallel `grite db stats`, `grite doctor`, or
context-index commands can legitimately report `db_busy`.

`grite db stats --json` exposes these operator signals:

- `size_bytes`: local projection size
- `event_count` and `issue_count`: materialized state cardinality
- `events_since_rebuild`: WAL events replayed since the last local rebuild
- `days_since_rebuild`: age of the last local rebuild, when recorded
- `rebuild_recommended`: canonical machine-readable rebuild signal

Use these conservative thresholds:

- Snapshot: create or allow a snapshot with `grite snapshot create` when WAL
  growth since the latest snapshot exceeds 10,000 events or the latest snapshot
  is older than 7 days.
- Projection rebuild: run `grite doctor --fix --json` or `grite rebuild` when
  `rebuild_recommended` is `true`, `events_since_rebuild` is 10,000 or more,
  `days_since_rebuild` is 7 or more, `size_bytes` is 512 MiB or more, or a crash
  left the projection unreadable.
- Snapshot GC: run `grite snapshot gc` during maintenance windows after snapshot
  churn, before large syncs, or when old snapshots are no longer useful locally.
- Daemon lock recovery: if DB busy/locked symptoms persist for more than 60s,
  check `brat daemon status --json` and `grite daemon status --json`; stop only
  stale or unwanted daemons, then rerun Brat doctor.
- Session staleness: Brat heartbeats every 30s and treats sessions as stale
  after 5m by default (`stale_session_ms = 300000`).

Do not delete the whole `.git/grite` directory or rewrite `refs/grite/*`.

## Sync

- `grite sync --pull` fetches `refs/grite/*`
- `grite sync --push` pushes `refs/grite/*`

If push is rejected, the client rebases by creating a new WAL commit parented to the remote head.

## Multi-agent concurrency (same repo or remote)

Concurrent agents are supported with a few strict rules:

- WAL appends are safe and monotonic. Locally, `git update-ref` is atomic: if two agents race to advance `refs/grite/wal`, one wins and the other must re-read the new head and append again.
- The local materialized view must not be shared across processes. `sled` is single-writer and not multi-process safe. Use per-agent data dirs under `.git/grite/actors/<actor_id>/` (recommended).
- Remote push races are common. On non-fast-forward push rejection, the client must fetch, re-append on the new head, and retry.

Retry rule (spec-grade):

- On WAL append failure (local race or remote non-fast-forward), the client MUST: read head -> create a new append commit on that head -> retry push with bounded exponential backoff.

## Snapshots

- `grite snapshot create` creates a monotonic snapshot ref
- `grite snapshot gc` prunes old snapshots (local policy)

Snapshots never change WAL history; they are purely a rebuild accelerator.
