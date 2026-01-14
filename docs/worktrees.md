# Worktrees

Each polecat (agent) works in its own git worktree. Metadata never touches worktrees and is stored in `.git/grit/` and `refs/grit/*`.

## Layout

- Worktrees: `.grit/worktrees/polecat-<n>` (gitignored)
- Main repo remains clean unless code edits are made

## Guarantees

- `git status` stays clean in all worktrees for metadata
- Task/memory state lives only in `refs/grit/*` and `.git/grit/`
- No tracked JSON or branch-based coordination state
