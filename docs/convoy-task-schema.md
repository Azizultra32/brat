# Convoy and task schema (Grite-backed)

This doc defines the minimal schema Brat uses to represent convoys and tasks in Grite. The goal is a stable, queryable structure for automation.

## IDs

ID formats are defined in `docs/canonical-spec.md`.

IDs are used in labels and comments for stable linkage.

## Convoy issue

Convoys are Grite issues with these required labels:

- `type:convoy`
- `convoy:<convoy_id>`
- `status:active|paused|complete|failed`

Recommended fields in the issue body:

```
Title: <convoy title>
Goal: <one-line objective>
Base commit: <git sha>
Policy: <merge policy summary>
Owner: <actor_id or handle>
```

## Task issue

Tasks are Grite issues with these required labels:

- `type:task`
- `task:<task_id>`
- `convoy:<convoy_id>`
- `status:queued|running|blocked|needs-review|merged|dropped`

Optional labels (see `docs/label-glossary.md` for canonical values):

- `priority:P0|P1|P2`
- `agent:todo`
- `assignee:<actor_id>`
- `engine:<name>`
- `needs-ack`
- `to:<actor_id>`
- `urgency:low|med|high`
- `merge:queued|running|failed|succeeded`

Merge label transitions follow `docs/merge-policy.md`.

Recommended fields in the issue body:

```
Title: <task title>
Paths: <comma-separated paths>
Constraints: <brief constraints>
Acceptance: <tests or checks>
Notes: <extra context>
```

## Linking convoys to tasks

- The canonical link is the `convoy:<convoy_id>` label on each task issue.
- The convoy issue should include a checklist of task IDs (optional).

## Status transitions

Status is managed via `status:*` labels only. Labels should be updated atomically by the harness.

Merge pipeline state is tracked via `merge:*` labels. On successful integration, set `status:merged` and remove `merge:*` labels.

## Naming conventions

- Labels are lowercase with `:` separators.
- IDs are short and stable; do not reuse IDs.
- Canonical labels are listed in `docs/label-glossary.md`.
