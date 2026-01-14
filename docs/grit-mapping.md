# Brat to Grit mapping

This doc describes how the harness maps its concepts onto Grit issues, comments, labels, and locks. It is an initial mapping and can evolve.

## Core entities

- Town (repo): `brat init` enables the repo and creates the WAL + local state.
- Convoy (bundle of tasks): represented as a Grit issue with label `type:convoy`.
- Task: represented as a Grit issue with label `type:task`.
- Task membership: task issues include label `convoy:<id>` (or a `LinkAdded` to the convoy issue).
- Session: represented as a Grit comment on the task issue (spawn, heartbeat, exit).
  - Session comments follow `docs/session-event-schema.md`.

## Labels and state conventions

Canonical labels are defined in `docs/label-glossary.md`. ID formats are defined in `docs/canonical-spec.md`.

- Identity:
  - `convoy:<convoy_id>` on convoy and task issues
  - `task:<task_id>` on task issues
- Convoy status:
  - `status:active`
  - `status:paused`
  - `status:complete`
  - `status:failed`
- Task status:
  - `status:queued`
  - `status:running`
  - `status:blocked`
  - `status:needs-review`
  - `status:merged`
  - `status:dropped`
- Ownership:
  - `assignee:<actor_id>` (or Grit assignees when supported)
- Engine:
  - `engine:<name>` for engine type (codex, claude, opencode)
- Session:
  - `session:spawned|ready|running|handoff|exit`
  - `session:polecat|crew`
- Priority and routing:
  - `priority:P0|P1|P2`
  - `agent:todo` for queueing
- Handoffs and acknowledgements:
  - `to:<actor_id>` for a directed handoff
  - `needs-ack` when a response is required
  - `ack:<actor_id>` once acknowledged
  - `urgency:low|med|high` for priority

## Handoff comment template

Use this structure in a comment when handing off work:

```
Summary:
Requested action:
Context:
Acceptance checks:
Deadline:
```

Guidelines:

- Add `to:<actor_id>` and `needs-ack` labels for directed handoffs.
- Add `urgency:low|med|high` to signal priority.
- Add `ack:<actor_id>` when the recipient confirms receipt.

## Roles to Grit actions

- Mayor
  - Creates convoy issue and task issues
  - Applies `convoy:<id>` labels to tasks
  - Sets status labels and assigns tasks

- Witness
  - Spawns sessions, posts comments on task issues
  - Updates `status:*` labels based on progress

- Refinery
  - Posts merge results and links
  - Updates task state to `merged` or `needs-review`

- Deacon
  - Cleans up stale locks
  - Rebuilds projections if needed
  - Syncs refs

## Locks

Locks are stored in `refs/grit/locks/*` and used for coordination.

- Path lock: `path:<path>`
- Task lock: `issue:<id>`
- Repo lock: `repo:global`

The harness should acquire a lock before risky edits and release it on completion.

## Worktrees

- Each agent uses its own worktree and actor data dir
- All coordination state stays in `refs/grit/*` and `.git/grit/`

## Optional enhancements

- A convoy issue can store a structured checklist in the body
- Task issue templates can encode acceptance criteria and test commands
- A `convoy:<id>` label can be replaced by a dedicated Link event when needed
- Mailbox primitives are expressed as labels + comments instead of a separate subsystem
