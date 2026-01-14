# Brat daemon (bratd)

`bratd` is the harness supervisor. It is **on by default** and can manage multiple repositories concurrently. It coordinates roles, tmux control room UX, and orchestration loops while delegating state to Grit.

## Responsibilities

- Role supervision for Mayor/Witness/Refinery/Deacon
- Tmux control room setup and session management
- Periodic reconciliation (session adoption, cleanup)
- Worktree lifecycle management
- Delegates persistence and sync to Grit (`grit`/`gritd`)

## Multi-repo behavior

- One `bratd` process can manage multiple repo roots.
- Each repo has its own role loops and worktree pool.
- Actor identity is resolved per repo via Grit actor directories.

## Interaction with gritd

- `bratd` depends on Grit as the source of truth.
- If `gritd` is running, `bratd` uses it for fast queries.
- If `gritd` is absent, `bratd` uses the Grit CLI directly.

## Failure behavior

- If a role loop crashes, `bratd` restarts it and posts a health note.
- If Grit is unreachable, `bratd` degrades gracefully and reports status.

## Control room (tmux)

- Session: `brat`
- Windows: `mayor`, `witness`, `refinery`, `deacon`, `sessions`
- Each window shows live status and role-specific logs.
