Yes: **you point it at any existing git repo** and it becomes “town-enabled” by creating **repo-local, non-working-tree metadata** (Gems WAL refs + a local sled DB). It doesn’t need your repo to adopt a framework or add tracked files.

Here’s how it works end-to-end.

## What happens when you “enable” a repo

### 1) Initialize Gems metadata (no tracked files)

In the target repo, we create:

* **Git refs** for the append-only WAL (authoritative history):

  * `refs/gems/wal/<actor>/<stream>`
  * optional convenience refs: `refs/gems/heads/<convoy>`
* A **local state directory** (ignored by git):

  * `.gems/` (or `.gastown/`)
  * contains **sled** DB + runtime sockets/logs
* A `.gitignore` entry *optional* (we can avoid touching it by using `.git/info/exclude`).

Nothing is added to your working tree unless you explicitly export something.

### 2) Create (or attach) a Town for that repo

A “town” is just: “this repo + a set of convoys + sessions.”

* `gt init` records a `TownInitialized` event into the WAL ref and builds the initial sled projection.
* If the WAL already exists (someone else already “town-enabled” it), `gt init` becomes “attach”: it fetches and replays.

## Daily usage flow

### A) Start a convoy (bundle of tasks)

You do:

* `gt convoy create ...`

This writes an event to the WAL ref:

* `ConvoyCreated { id, title, base_commit, policy }`

Then the projection updates in sled, so queries are fast:

* `gt convoy list`
* `gt convoy status <id>`

### B) Spawn workers (polecats) safely in worktrees

When you run:

* `gt swarm start --n 6 --convoy X`

Witness (either as a one-shot CLI action or via the daemon) will:

* create N **git worktrees** under `.gems/worktrees/`
* spawn N agent sessions pointing at those worktrees
* emit events:

  * `WorktreeCreated`
  * `SessionSpawned`
  * `TaskAssigned`

Your main working tree stays clean. The agents do real edits in their worktrees.

### C) Merge results via Refinery

Refinery watches for “task ready” signals (events), then:

* rebases/merges or opens PRs depending on policy
* emits:

  * `MergeAttempted`
  * `MergeSucceeded` / `MergeFailed`
  * `TaskStatusChanged`

Again: no branch-shape heuristics. “State” is whatever events say happened.

## How it works with collaboration (multiple machines/users)

This is the key: **Git is only transport.**

* Everyone pulls/pushes `refs/gems/wal/*`
* Merging histories is trivial because:

  * events are content-addressed (EventId = hash)
  * applying events is idempotent (`events_seen` in sled)
  * set-union merge = “replay everything you haven’t seen”

So you can:

* work offline
* later push your WAL refs
* others fetch and replay → consistent town state

## How to “point it at any repo” in practice

### Case 1: Local repo only (no remote needed)

Works fully offline:

* WAL refs exist locally
* sled lives in `.gems/`
* workers run in local worktrees

### Case 2: Shared repo with remote

You configure one remote for WAL distribution (usually the same origin):

* fetch/push includes `refs/gems/wal/*`

This can be:

* automatic by daemon (accelerator)
* or explicit:

  * `gt sync push`
  * `gt sync pull`

No need to change the repo’s main branches.

### Case 3: Repo you can’t push to (read-only upstream)

Still works:

* you store WAL refs in *your own* remote (fork, private bare repo, or even local-only)
* the code changes can go via your fork/PR flow

## What about “existing workflows”? (PRs, CI, trunk-based)

It’s compatible because:

* it only creates worktrees and normal code edits
* it can either:

  * merge locally to your branch, or
  * create PRs in GitHub/GitLab (optional integration)

Gems metadata stays out of the way.

## Minimal mental model

Think:

* **Repo code**: normal git history (your project)
* **Gems WAL refs**: immutable “facts about coordination”
* **Sled**: a cache/materialized view you can delete/rebuild anytime
* **Daemon**: a convenience layer, not correctness

## What the user actually does (typical loop)

1. `gt init`
2. `gt convoy create`
3. `gt tasks add ...` (or import from issues)
4. `gt swarm start`
5. `gt status` / `gt feed --follow`
6. `gt refinery merge` (or automatic policy)
7. `gt sync push`

That’s it.

If you want, I can write the exact CLI surface for “point at a repo” (init/attach/import/sync/worktrees) plus the **git refspecs** you’d add so `git fetch`/`git push` moves `refs/gems/wal/*` cleanly without touching branches.

