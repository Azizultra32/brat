# Naming and pitch

## Recommended name

Brat

### Why

- Distinct and memorable without being corporate
- Short and easy to type
- Works as the harness name on top of Grit
- Gastown was about agents; Brat is about reliable coordination

## Alternatives

1. LedgerTown
2. Factum
3. Union
4. Anvil

## One-line pitch

Brat is an autonomous multi-agent coding harness backed by Grit, an immutable event ledger in Git refs, so agents can coordinate safely, offline, and at scale.

## 30-second pitch

Modern coding agents fail not because they cannot write code, but because they cannot coordinate safely. Existing tools treat Git branches and files as mutable state, which breaks under concurrency, worktrees, and multi-machine use.

Brat fixes this by separating facts from views with Grit:

- Agents emit immutable events stored in Git refs.
- State is derived locally from those events.
- Merges are set-union, repairs are monotonic, and `gritd` is optional (while `bratd` runs by default for UX).

Same harness. Correct substrate.

## Taglines

- Immutable facts. Autonomous agents.
- Git for facts, not guesses.
- Coordination without corruption.
- Agents you can actually trust.
