Below is a **Rust-first spec for “Gastown Improved”** assuming:

* You have a **repo-local orchestration system** like Gastown (roles: Mayor/Witness/Refinery/Deacon, polecats, etc.).
* You replace Beads with **Gems** as the substrate:

  * **Git refs as an append-only WAL** (transport + audit),
  * **Sled as the derived/materialized state** (fast queries, projections),
  * **Never dirty the working tree** for town metadata,
  * **Daemon is optional** (accelerator only),
  * **Repair is monotonic** (replay/rebuild/add-only).
* IPC = **async-nng**, encoding = **rkyv**.

---

# 1) Product goal and invariants

## Goals

1. Multi-agent coding orchestration that is **boring & reliable**:

   * deterministic state,
   * safe merging across worktrees,
   * offline-first,
   * agent-friendly CLI and JSON outputs.
2. Preserve Gastown strengths:

   * role topology (Mayor/Witness/Refinery/Deacon),
   * tmux “control room” UX,
   * git-backed, repo-local persistence.
3. Eliminate Beads failure modes:

   * no tracked jsonl files in working tree,
   * no “doctor --fix” rewriting published history,
   * no daemon-required correctness.

## Hard invariants

* **No metadata files in the working tree** (unless explicitly exported).
* **All authoritative facts are immutable events**.
* **All merges are set-union of events** (idempotent).
* **Daemon optional**: every CLI command is a complete transaction.
* **Repair is monotonic**: rebuild projections, never rewrite published WAL refs.

---

# 2) High-level architecture

## Components

### A) `gt` CLI (always correct)

* Writes/reads events (WAL refs)
* Updates local projections (sled)
* Can run without daemon
* Default outputs are structured (`--json`), interactive only with explicit flags

### B) `gtd` daemon (accelerator)

* Pub/sub notifications (nng)
* Background sync/fetch/push of WAL refs
* Warm caches (sled)
* Watches processes (role sessions) and emits lifecycle events
* Can be stopped without breaking correctness

### C) Git WAL layer (Gems)

* Append-only events stored under `refs/gems/wal/<actor>/<stream>`
* Optional “heads” pointers for convenience: `refs/gems/heads/<convoy>` etc.
* Pull/push transfers refs; merge is union by replay

### D) Projection store (sled)

* Materialized views: convoys, tasks, sessions, leases, mailboxes, audit timeline
* Deterministic projection from events
* Rebuildable at any time (“doctor” just rebuilds)

---

# 3) Data model: events, streams, projections

## Event encoding

* **Binary canonical event**: `rkyv`-archived struct (fast, deterministic)
* **Hash**: `blake3( archived_bytes )` → `EventId`
* Optional JSON envelope for human inspection/debug export, but not authoritative.

### Event envelope (conceptual)

```rust
struct Event {
  id: [u8; 32],          // blake3
  ts: i64,               // unix millis
  actor: ActorId,        // "mayor", "witness", "polecat:7", "user:dipankar"
  kind: EventKind,       // enum
  convoy: Option<ConvoyId>,
  causality: Option<[u8;32]>, // parent event id (optional)
  payload: Payload,      // rkyv
}
```

## WAL storage in git refs

* Events are appended to a **pack-like log** per stream:

  * `refs/gems/wal/<actor>/<yyyymmdd>` or `<stream-id>`
* Each append updates the ref to point at a new blob/tree that contains:

  * `segment header`
  * concatenated `rkyv` event bytes
  * per-segment index (offsets) for fast scanning

**Why segments:** efficient append, efficient fetch, efficient replay.

## Projections (sled trees)

Use separate sled trees (namespaces):

* `events_seen`: `EventId -> bool`
* `convoy_state`: `ConvoyId -> ConvoyView`
* `tasks`: `TaskId -> TaskView`
* `sessions`: `SessionId -> SessionView`
* `leases`: `LeaseKey -> LeaseView`
* `mailbox`: `MailboxKey -> Vec<MessageRef>`
* `timeline`: `ts|eventid -> EventSummary`
* `indexes/*`: secondary indexes for queries (by status, by assignee, etc.)

All views are derived: delete sled and replay WAL → identical state.

---

# 4) Core domain objects

## Convoy

A convoy is the unit of orchestration and review.

* `ConvoyCreated { title, repo_commit, goal, policy }`
* `ConvoyTaskAdded { task_id, spec, labels }`
* `ConvoyAssigned { task_id, assignee }`
* `ConvoyStatusChanged { status }` (active/paused/complete/failed)

## Task

Tasks are stable identifiers; polecats may come and go.

* `TaskSpec`: repo paths, constraints, test commands, acceptance checks
* `TaskStatus`: queued/running/blocked/needs-review/merged/dropped

## Session

Represents a running process (Claude Code / Codex / “engine plugin”).

* `SessionSpawned { session_id, role, engine, worktree, pid?, tmux_pane? }`
* `SessionHeartbeat { cpu, mem, last_output_hash }`
* `SessionExited { code, reason, last_lines_ref }`

## Mailbox / handoff

* `MessageSent { from, to, convoy, content_ref, urgency }`
* `MessageAcked { message_id }`

## Lease locks (exclusive checkout)

Leases are first-class, time-bound locks:

* `LeaseAcquired { key, holder, ttl_ms }`
* `LeaseRenewed { key, holder, ttl_ms }`
* `LeaseReleased { key, holder }`
* `LeaseExpiredObserved { key, observed_by }` (projection can expire locally)

Lease keys can be:

* repo path (`path:src/foo.rs`)
* task (`task:<id>`)
* worktree (`wt:<name>`)

**Rule:** leases are “soft” but enforceable by policy. CLI refuses conflicting writes unless `--force`.

---

# 5) Role mapping: what each role *does* in the improved design

## Mayor (control plane)

* Creates convoys, adds tasks, assigns, monitors
* Never directly “manages processes” beyond user UX
* Reads from projections; writes high-level intent events

## Witness (worker controller)

* Responsible for turning intent into worker sessions:

  * decides how many polecats,
  * spawns them via engine plugin,
  * monitors their heartbeats,
  * emits lifecycle events.
* If daemon absent: `gt witness run` is just a CLI subcommand that does it once.

## Refinery (integration controller)

* Consumes completed task outputs and produces merge events:

  * `PRProposed`, `MergeAttempted`, `MergeSucceeded`, `MergeFailed`
* Owns the “merge queue” policies:

  * maximum parallel merges,
  * rebase strategy,
  * required checks.

## Deacon (janitor/reconciler)

* Periodically:

  * expires leases,
  * detects orphan sessions (no heartbeat),
  * replays WAL to refresh projections,
  * syncs refs with remotes,
  * emits health events.

---

# 6) Engine abstraction (Claude Code, Codex, OpenCode, etc.)

Define a strict plugin trait:

```rust
trait Engine {
  fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult>;
  fn send(&self, session: SessionHandle, input: EngineInput) -> Result<()>;
  fn tail(&self, session: SessionHandle, n: usize) -> Result<Vec<String>>;
  fn stop(&self, session: SessionHandle, how: StopMode) -> Result<()>;
  fn health(&self, session: SessionHandle) -> Result<EngineHealth>;
}
```

Implementations:

* `engine-claude-code`
* `engine-codex-cli`
* `engine-opencode`
* `engine-shell` (for tests / simulation)

All engine interactions are wrapped in:

* bounded timeouts
* captured outputs
* structured errors → emitted as events

---

# 7) IPC (async-nng) and service API

## Sockets

* `ipc:///tmp/gtd.sock` (or repo-local `.gems/run/gtd.sock`)
* Patterns:

  * **REQ/REP** for commands
  * **PUB/SUB** for events + notifications

## API messages (rkyv)

* `Request::Query { kind, filters }`
* `Request::Command { cmd }`
* `Response::Ok { payload }`
* `Response::Err { code, message, details }`
* `Notification::EventApplied { event_id, summary }`
* `Notification::Health { role, status }`

CLI behavior:

* If daemon present: prefer IPC for speed.
* If not: do direct git+sled operations locally.

---

# 8) CLI contract (agent-first)

## Non-interactive by default

* Every command supports `--json`
* Commands never hang by default:

  * streaming requires `--follow`
  * waiting requires `--wait --timeout <ms>`

Examples:

* `gt convoy create --title ... --json`
* `gt convoy status <id> --json`
* `gt session tail <id> --lines 200 --json`
* `gt events scan --since ... --json`
* `gt doctor --check` (fast)
* `gt doctor --rebuild` (explicit, monotonic)

## “Feed” semantics fixed

* `gt feed` becomes:

  * `gt feed --once` (snapshot)
  * `gt feed --follow` (stream)
  * `gt feed --timeout 5000` (bounded follow)

---

# 9) Doctor and repair (monotonic)

## Doctor modes

* `doctor --check`:

  * verify git refs exist
  * verify sled schema version
  * verify projections consistent (optional spot checks)
* `doctor --rebuild`:

  * delete sled projections
  * replay WAL
  * produce a report
* `doctor --propose`:

  * suggests actions (e.g., “session orphaned; consider stop”)
  * never force-resets refs

**Never:**

* rewriting WAL
* force pushing “fixed” state
* destructive repairs without explicit user command and audit event.

---

# 10) Git workflow and worktree safety

## Worktrees

* Each polecat gets its own worktree:

  * `worktrees/polecat-<n>`
* Town metadata never touches worktrees:

  * no `.jsonl` tracked
  * no “sync branch health”
* Events are in refs; worktrees remain clean unless the agent edits code.

## Multi-machine collaboration

* Share via git remotes:

  * push/pull `refs/gems/wal/*`
* Merge = union of WAL segments:

  * projection ensures idempotency (`events_seen`)

---

# 11) Repository layout

```
gastown-rs/
  crates/
    gt-cli/
    gtd/
    gems-core/
      event/
      wal-git/
      projection-sled/
      leases/
      mailbox/
      engine/
    engines/
      engine-claude-code/
      engine-codex/
      engine-shell/
  docs/
    architecture.md
    event-model.md
    cli.md
    roles.md
    operations.md
  tests/
    integration/
      swarm_merge_queue.rs
      lease_exclusivity.rs
      daemon_optional.rs
```

---

# 12) Acceptance tests derived from the pain points

1. **Worktree-safe metadata**

* Create 3 worktrees, run town ops from each → `git status` clean in all worktrees.

2. **Union merge of WAL**

* Two machines produce events offline → push/pull → projections converge deterministically.

3. **Daemon optional**

* Stop daemon → CLI can still create convoy, assign tasks, tail sessions (via engine), rebuild projections.

4. **No silent death**

* Kill an engine process → `SessionExited` emitted with exit code + last lines ref.

5. **Leases**

* Two polecats attempt to claim same path/task → second is blocked unless lease expired or `--force`.

6. **Doctor monotonic**

* Corrupt sled store → `doctor --rebuild` restores state from WAL; no git rewriting occurs.

---

# 13) Implementation notes (practical choices)

* **rkyv** for all internal IPC and WAL event bytes (speed, deterministic)
* **serde_json** only for CLI output / debug export
* **blake3** for event ids and log segment integrity
* **sled** schema versioning: store `schema_version` key; migrations are replay-based
* **git plumbing**:

  * use `git2` or shelling out to `git` with strict args
  * but WAL writes should be atomic: write objects then update ref

---

# 14) What you get vs today’s Gastown

* Same mental model: **roles + tmux cockpit + swarming**
* Different substrate: **events in refs + derived state in sled**
* The reliability leap comes from:

  * no worktree conflicts on metadata,
  * no destructive “fixes,”
  * deterministic replay,
  * daemon no longer required.

---

If you want the next step, I’ll produce **(a)** the exact `EventKind` enum list (v1), **(b)** the sled key schema, and **(c)** a concrete “WAL segment in git objects” format (tree/blob layout + atomic update algorithm) so engineering can start immediately.

