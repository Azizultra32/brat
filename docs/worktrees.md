# Worktrees

Each polecat (agent) works in its own git worktree. Metadata never touches worktrees and is stored in `.git/grite/` and `refs/grite/*`.

## Layout

- Worktrees: `.grite/worktrees/polecat-<n>` (gitignored)
- Main repo remains clean unless code edits are made

## Guarantees

- `git status` stays clean in all worktrees for metadata
- Task/memory state lives only in `refs/grite/*` and `.git/grite/`
- No tracked JSON or branch-based coordination state
