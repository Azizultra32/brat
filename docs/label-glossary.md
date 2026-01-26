# Label glossary

This doc defines the canonical labels used by Brat when writing to Grite. Labels are lowercase with `:` separators unless noted.

## Identity

- `type:convoy`
- `type:task`
- `convoy:<convoy_id>`
- `task:<task_id>`
- `repo:<name>`

## Convoy status

- `status:active`
- `status:paused`
- `status:complete`
- `status:failed`

## Task status

- `status:queued`
- `status:running`
- `status:blocked`
- `status:needs-review`
- `status:merged`
- `status:dropped`

## Session state and type

- `session:spawned|ready|running|handoff|exit`
- `session:polecat|crew`

## Role state and health

- `role:idle|active|degraded|recovering`
- `health:ok|warn|fail`

## Merge pipeline

- `merge:queued`
- `merge:running`
- `merge:failed`
- `merge:succeeded`

## Ownership and engine

- `assignee:<actor_id>`
- `engine:<name>`

## Priority and routing

- `priority:P0|P1|P2`
- `agent:todo`
- `to:<actor_id>`
- `needs-ack`
- `ack:<actor_id>`
- `urgency:low|med|high`

## Lock resources

Lock resource strings use namespace prefixes. See `docs/locking.md` for full details.

- `repo:global` - repo-wide lock
- `path:<path>` - file or directory lock
- `issue:<id>` - issue-scoped lock
