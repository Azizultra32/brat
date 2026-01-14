# Multi-repo convoys (mirrored)

Brat supports multi-repo projects by mirroring convoys across repos. Each repo has its own convoy issue, but all share the same `convoy_id` so context can be aggregated.

## Model

- Each repo has its own WAL and projections.
- A convoy is created in each repo with the same `convoy_id` and matching title/goal/policy.
- Tasks are repo-local but linked by the shared `convoy_id` label.
- Optional label `repo:<name>` can be added for aggregation.

## Default behavior

- Brat operates on the current repo only unless `--all-repos` or `--repo <path>` is specified.

## Benefits

- Local context stays in each repo (offline friendly).
- Aggregation is possible via shared `convoy_id`.
- No cross-repo transaction needed.

## Default behavior

- `brat convoy create --mirror --repos <paths>` creates the convoy in all specified repos.
- The same `convoy_id` is used everywhere.
- Each repo’s convoy issue is minimal and stable; evolving context goes in task comments.

## Add a repo later (on the fly)

- `brat convoy add-repo <convoy_id> --repo <path>`
  - Creates a convoy issue in the new repo with the same `convoy_id`.
  - Adds `repo:<name>` label if configured.

## Task creation

- `brat task add --convoy <convoy_id> --repo <path> --title ...`
- Tasks only live in the repo that owns the code.

## Consistency guidance

- Keep convoy body fields stable (Title/Goal/Policy).
- Use comments on tasks for evolving context.
- If a convoy is closed, close it in each repo with `status:complete`.

## Aggregation examples

List all tasks for a convoy across repos:

```
brat task list --label convoy:<convoy_id> --all-repos --json
```

Summarize per-repo status:

```
brat status --convoy <convoy_id> --all-repos --json
```
