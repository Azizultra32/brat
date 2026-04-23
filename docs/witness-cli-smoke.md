# Witness CLI smoke test

This smoke test verifies that `brat witness` can run an unrelated task through a coding engine without falsely promoting incomplete work to review.

## Local Codex path

Use the local Codex CLI engine through Witness:

```sh
brat convoy create --json \
  --title "Unrelated Codex guardrail smoke" \
  --body "CLI smoke test for an unrelated task using the Codex engine."

brat task create --json \
  --convoy <convoy_id> \
  --title "Create unrelated CLI probe note" \
  --body $'Allowed paths:\n- notes/unrelated_codex_probe.txt\n\nCreate the allowed file with exactly three lines:\n1. BRAT unrelated CLI probe\n2. engine=codex\n3. status=ok\n\nDo not modify any other files.'

brat witness --no-daemon run --once --engine codex --json
```

The Codex adapter invokes the installed `codex` binary, not Codex Cloud:

```sh
codex exec \
  --dangerously-bypass-approvals-and-sandbox \
  --json \
  --cd <task-worktree> \
  "<task prompt>"
```

## Expected safe outcomes

A task may move to `needs-review` only when all of these are true:

- The worker produced at least one commit on `task-<task_id>`.
- The task branch changes at least one file relative to the base branch.
- If the task body contains `Allowed paths:`, all changed files are inside those paths.

If the worker exits but produces no commit, the task must be `blocked`, not `needs-review`.

If the worker changes files outside the allowed paths, the task must be `blocked` with a guardrail comment naming the out-of-scope paths.

## Orphan recovery rule

Witness may find a `running` task with no live session but with an existing `task-<task_id>` branch. Branch existence is not enough to prove useful work.

Recovery must validate the branch before changing task state:

- Valid branch output becomes `needs-review`.
- Missing commits, empty diffs, or out-of-scope diffs become `blocked`.
- The recovery comment must explain the decision.

This prevents failed or no-op workers from looking reviewable after a one-shot Witness run or process crash.

## Verification commands

```sh
brat status --json
git rev-list --count main..task-<task_id>
git diff --name-status main...task-<task_id>
```

For a no-op Codex run, expected status is `blocked` and expected commit count is `0`.

For a valid run, expected status is `needs-review`, commit count is greater than `0`, and `git diff --name-status` only lists allowed paths.
