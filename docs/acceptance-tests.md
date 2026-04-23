# Acceptance tests

1. Worktree-safe metadata
   - Create multiple worktrees, run Brat ops from each, and `git status` is clean.

2. Union merge of WAL
   - Two machines produce events offline, then push/pull; projections converge.

3. Daemon optional
   - With `grited` stopped, `brat` can still create tasks, comment, and rebuild projections.

4. No silent death
   - Kill an engine process; harness posts a task comment with exit code and last logs.

5. Locks
   - Two agents attempt to claim the same path; the second is blocked unless expired or `--force`.

6. Doctor monotonic
   - Corrupt local sled store; `brat doctor --rebuild` restores state without rewriting refs.

7. Witness branch guardrail
   - Run an unrelated task through `brat witness --engine codex`; no-op workers and orphaned task branches without commits are blocked, while valid task branches with allowed-path changes become `needs-review`.
