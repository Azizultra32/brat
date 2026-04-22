# Merge policy (Refinery)

This doc defines how the Refinery manages merge state using labels and comments.

## Labels

- `merge:queued` when a task is ready for integration
- `merge:running` while a merge attempt is in progress
- `merge:failed` when a merge attempt fails
- `merge:succeeded` when integration completes

## Required checks

- Defined in `.brat/config.toml` under `[refinery].required_checks`.
- If checks fail, the task moves to `merge:failed`.
- If checks are missing or still pending, Refinery requeues the task at `merge:queued`.
- Refinery integrates into `[refinery].target_branch`, which defaults to `"auto"` and resolves from local `origin/HEAD`, then `git remote show origin`.
- If neither source yields a default branch, Refinery fails closed and requires an explicit branch in config.

## Retry policy (default)

- Max retries: 2 (configurable via `[refinery].merge_retry_limit`)
- Backoff: exponential, starting at 2 minutes and doubling up to 32 minutes
- Retry count is recorded in the merge comment block

## Merge comment format

Each merge attempt posts a structured comment on the task issue:

```
[merge]
attempt = 1
strategy = "rebase"
pr = "https://..." # optional
result = "failed"
reason = "conflicts" # or "checks_failed", "unknown"
merge_commit = null
[/merge]
```

## State transitions

- `merge:queued` -> `merge:running` when a merge attempt starts
- `merge:running` -> `merge:succeeded` on success (also set `status:merged`)
- `merge:running` -> `merge:failed` on failure
- `merge:failed` -> `merge:queued` on retry

On success:

- Remove `merge:*` labels
- Set `status:merged`

## PR linking

If a PR is created, include the PR URL in the merge comment block.
