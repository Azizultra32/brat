# Session event schema

Brat records session lifecycle events as structured comments plus labels on the task issue.

## Labels

- Session state: `session:spawned|ready|running|handoff|exit`
- Session type: `session:polecat|crew`
- Engine: `engine:<name>`

## Comment format

Use a machine-readable block in a comment so automation can parse it. ID formats are defined in `docs/canonical-spec.md`.

```
[session]
state = "running"
session_id = "s-20250114-7b3d"
role = "witness"
session_type = "polecat"
engine = "codex"
worktree = ".grit/worktrees/polecat-3"
pid = 12345
started_ts = 1700000000000
last_heartbeat_ts = 1700000005000
exit_code = null
exit_reason = null
last_output_ref = "sha256:..."
[/session]
```

## Required fields

- `state`
- `session_id`
- `role`
- `session_type`
- `engine`
- `worktree`
- `started_ts`

## Optional fields

- `pid`
- `last_heartbeat_ts`
- `exit_code`
- `exit_reason`
- `last_output_ref`

## Allowed values

- `role`: `mayor|witness|refinery|deacon|user`
- `session_type`: `polecat|crew`

For crew sessions, set `role = "user"` and `session_type = "crew"`.

## Heartbeats

- Update `last_heartbeat_ts` at a fixed cadence (default 30s).
- Avoid comment spam by updating the most recent session comment when possible.

## Exit semantics

On exit, set:

- `state = "exit"`
- `exit_code`
- `exit_reason` (signal, timeout, crash, user stop)
- `last_output_ref` (hash or pointer to logs)

## Parsing rules

- The block is delimited by `[session]` and `[/session]`.
- Key/value pairs use `key = "value"` with quoted strings; integers are bare.
- `null` is allowed for unset fields (for example `exit_code = null`).
- Unknown keys are ignored; future-compatible.
