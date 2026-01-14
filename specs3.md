Here’s a **clean naming + pitch set** that’s honest, non-hypey, and technically defensible for *your* audience (infra + agents + serious devs).

I’ll give you **one primary name** (recommended), plus **alternatives**, then **the pitch** in three levels.

---

## Recommended name

## **Gems Town**

*(often just **Gems** in docs, “Gems Town” when contrasted with Gastown)*

### Why this works

* **Continuity without baggage**

  * Signals lineage from Gastown without inheriting Beads’ failure modes.
* **Correct metaphor**

  * Gems = immutable facts, valuable, append-only.
  * Town = autonomous coordination layer.
* **Not cute, not corporate**

  * Feels like a systems tool, not a SaaS dashboard.
* **Expandable**

  * Gems Core, Gems WAL, Gems Town, Gems CLI, Gems Daemon.

> Gastown was about *agents*.
> **Gems Town is about *facts* agents can trust.**

---

## Strong alternatives (ranked)

### 2) **LedgerTown**

* Very clear mental model
* Slightly more “enterprise”
* Less playful, more formal

### 3) **Factum**

* Latin for “that which has been done”
* Elegant, but abstract
* Needs explanation

### 4) **Union**

* Emphasizes merge-by-union semantics
* Might be too generic

### 5) **Anvil**

* Good metaphor (forging work)
* Loses the “append-only facts” clarity

If you want maximum clarity for engineers: **Gems Town** wins.

---

## The one-line pitch (this matters most)

> **Gems Town is an autonomous multi-agent coding harness that uses Git as an immutable event ledger—not a mutable database—so agents can coordinate safely, offline, and at scale.**

That sentence alone explains:

* what it is
* why it’s different
* why it fixes existing tools

---

## The 30-second pitch (README / intro talk)

> Modern coding agents fail not because they can’t write code, but because they can’t **coordinate safely**.
>
> Existing tools treat Git branches and files as mutable state, which breaks under concurrency, worktrees, and multi-machine use.
>
> **Gems Town fixes this by separating facts from views.**
>
> * Agents emit immutable events stored in Git refs.
> * State is derived locally from those events.
> * Merges are set-union, repairs are monotonic, and the daemon is optional.
>
> You get a swarm of agents that can:
>
> * work offline,
> * survive crashes,
> * coordinate across machines,
> * and never corrupt your repo.
>
> **Same harness. Correct substrate.**

---

## The “why we built this” pitch (Beads/Gastown-aware audience)

> Gastown proved that multi-agent coding needs structure: roles, swarms, merge queues, and a control room.
>
> But Beads showed the hard truth:
> **Git is a terrible mutable database.**
>
> Tracked JSONL files, branch health heuristics, destructive repair commands—these fail under real concurrency.
>
> **Gems Town keeps Gastown’s harness but replaces the substrate.**
>
> Git is used only as an append-only transport for immutable facts.
> All state is derived, rebuildable, and never rewritten.
>
> The result:
>
> * no dirty worktrees,
> * no dangerous “doctor” commands,
> * no daemon-required correctness,
> * no silent failures.
>
> Agents trust ledgers.
> Humans trust boring systems.
>
> **Gems Town is both.**

---

## The “brutally honest” positioning line

Use this when talking to serious engineers:

> **“Gastown showed the shape of the solution.
> Gems Town fixes the parts that break in production.”**

---

## Tagline options (pick one)

* **“Immutable facts. Autonomous agents.”**
* **“Git for facts, not guesses.”**
* **“Coordination without corruption.”**
* **“Agents you can actually trust.”**

---

## If you want, next step

I can:

1. Write the **README.md** opening section verbatim
2. Draft a **“Why not Beads?”** page that’s respectful but precise
3. Define the **v1 scope boundary** (what Gems Town *will not* try to do)

Just say which one.

