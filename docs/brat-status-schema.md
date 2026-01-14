# Brat status schema

This document defines the JSON shape returned by `brat status --json`.

## Top-level

```json
{
  "schema_version": 1,
  "generated_ts": 1700000000000,
  "repo_root": "/path/to/repo",
  "convoys": [ ... ],
  "tasks": { ... },
  "sessions": [ ... ],
  "merge_queue": { ... },
  "locks": [ ... ],
  "interventions": [ ... ]
}
```

Notes:

- JSON keys use `snake_case`.
- Status keys correspond to label suffixes with `-` converted to `_` (for example `needs-review` -> `needs_review`).

## Multi-repo (`--all-repos`)

When `--all-repos` is set, the response is:

```json
{
  "schema_version": 1,
  "generated_ts": 1700000000000,
  "repos": [
    {
      "repo_root": "/path/to/repo-a",
      "convoys": [ ... ],
      "tasks": { ... },
      "sessions": [ ... ],
      "merge_queue": { ... },
      "locks": [ ... ],
      "interventions": [ ... ]
    }
  ]
}
```

## Convoys

```json
{
  "convoy_id": "c-20250114-a2f9",
  "title": "...",
  "status": "active",
  "task_counts": {
    "queued": 3,
    "running": 2,
    "blocked": 1,
    "needs_review": 1,
    "merged": 0,
    "dropped": 0
  }
}
```

## Tasks summary

```json
{
  "total": 12,
  "by_status": {
    "queued": 3,
    "running": 2,
    "blocked": 1,
    "needs_review": 1,
    "merged": 4,
    "dropped": 1
  }
}
```

## Sessions

```json
{
  "session_id": "s-20250114-7b3d",
  "task_id": "t-20250114-3a2c",
  "role": "witness",
  "session_type": "polecat",
  "engine": "codex",
  "state": "running",
  "last_heartbeat_ts": 1700000005000
}
```

## Merge queue

```json
{
  "queued": 2,
  "running": 1,
  "failed": 1,
  "succeeded": 4
}
```

## Locks

```json
{
  "resource": "path:src/parser.rs",
  "owner": "<actor_id>",
  "expires_ts": 1700000000000
}
```

## Interventions

Interventions follow the schema in `docs/usability.md` (including `cognitive_prompt` and `recommended_actions`).
