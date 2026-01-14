# Naming and pitch

## Recommended name

Gems Town (short: Gems)

### Why

- Continuity with Gastown without Beads baggage
- Correct metaphor: coordination layer backed by immutable facts
- Not cute, not corporate
- Expandable naming (Gems Town harness backed by Grit substrate)
- Gastown was about agents; Gems Town is about facts agents can trust

## Alternatives

1. LedgerTown
2. Factum
3. Union
4. Anvil

## One-line pitch

Gems Town is an autonomous multi-agent coding harness backed by Grit, an immutable event ledger in Git refs, so agents can coordinate safely, offline, and at scale.

## 30-second pitch

Modern coding agents fail not because they cannot write code, but because they cannot coordinate safely. Existing tools treat Git branches and files as mutable state, which breaks under concurrency, worktrees, and multi-machine use.

Gems Town fixes this by separating facts from views with Grit:

- Agents emit immutable events stored in Git refs.
- State is derived locally from those events.
- Merges are set-union, repairs are monotonic, and the daemon is optional.

Same harness. Correct substrate.

## Taglines

- Immutable facts. Autonomous agents.
- Git for facts, not guesses.
- Coordination without corruption.
- Agents you can actually trust.
