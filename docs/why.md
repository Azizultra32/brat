# Why Brat (Grite-backed)

## What it avoids

1. Dirty working trees and phantom diffs
   - Metadata lives in `refs/grite/*` and `.git/grite/`, never in tracked files.

2. Destructive repair workflows
   - Repair is monotonic (rebuild projections), never rewrite WAL refs.

3. Daemon-required correctness
   - Every CLI command is a complete transaction; `grited` is optional and `bratd` is not required for correctness.

4. Branch-topology heuristics
   - Coordination state is not encoded in branches; merges are set-union of events.

5. Silent session failures
   - Harness posts structured updates via Grite comments and labels.

6. Blocking CLIs
   - Streaming and waiting are explicit; defaults are non-blocking.

## What this does not solve

- Engine brittleness (auth, rate limits, vendor instability)
- Real merge conflicts and flaky tests
- Prompt quality and task decomposition
- Human workflow integration (CI gates, PR conventions)

## Positioning lines

- Crisp claim: We avoid most Gastown issues by removing git-as-mutable-database and replacing it with an append-only WAL in refs plus rebuildable derived state.
- Brutally honest: Gastown showed the shape of the solution. Brat fixes the parts that break in production.
