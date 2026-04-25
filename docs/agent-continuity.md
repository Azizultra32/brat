# Agent Continuity

This repository now uses a local Codex continuity model based on real session artifacts, not manual copy-paste notes.

## Goals

- Keep agents addressable after they finish active work.
- Track when an agent should leave the active pool because its prompt load is getting too high.
- Preserve enough context for future sessions to recover what mattered without pretending we can inspect hidden provider-side state.

## What We Can Measure

Two local artifact sources matter:

- `~/.codex/sessions/**/*.jsonl`
  - Canonical session transcript.
  - Contains `session_meta`, `user_message`, and `token_count` events.
- `~/.codex/state_5.sqlite`
  - Thread registry with titles and nicknames.

The useful prompt-load signal is:

- `token_count.info.last_token_usage.input_tokens / token_count.info.model_context_window`

This is the latest turn's visible prompt size relative to the model context window.

## What We Do Not Claim

We do not have a first-class meter for the provider's hidden live context occupancy or KV-cache state.

That means:

- `tokens_used` from the state database is not the retirement metric. It is cumulative lifetime usage.
- `total_token_usage.total_tokens` in session logs is also cumulative.
- The retirement rule uses the latest prompt payload size, not cumulative spend.

## Pool Policy

Agent pool states:

- `active`
  - Below the retirement threshold.
  - Safe to reuse for continued work.
- `retired`
  - At or above the retirement threshold.
  - Keep the thread/session around, but do not treat it as part of the default reusable worker pool.
- `unknown`
  - No usable session telemetry yet.

Current retirement threshold:

- `70%`

## Current Named Agents

The named subagents tracked in the local pool, such as `Boyle`, `Einstein`, `Galileo`, `Pascal`, `Ptolemy`, and `Bernoulli`, are parked Codex threads from earlier work.

That means:

- they are addressable session histories,
- they are not necessarily executing work right now,
- their pool status tells us whether they are safe to reuse, not whether they are currently busy.

Liveness for background monitors is handled separately through the actual terminal processes, such as the `tmux` watcher session.

## Runtime Files

These files are generated under `~/.codex/` and are not repository artifacts:

- `context-pool.json`
  - Latest machine-readable pool snapshot.
- `context-pool-events.jsonl`
  - Transition log whenever a thread changes pool state.
- `session-handoff.md`
  - Human-readable handoff with recent user-message references and exact session log locations.
- `continuity-supervisor.json`
  - Machine-readable continuity protocol state including compaction count and caretaker requirements.
- `continuity-companion.md`
  - Human-readable continuity report that explicitly documents the documentation companion and caretaker slots.
- `continuity-supervisor-events.jsonl`
  - Transition log for compaction count, caretaker requirement, and main-thread pool-state changes.

Repository-local coding continuity artifacts are generated under `.brat/continuity/`:

- `project-continuity.json`
  - Machine-readable record of terminal-session interactions for the current project.
- `project-continuity.md`
  - Human-readable report with the interaction log first and the important-file inventory second.

## Grite's Role

Grite helps with durable project memory and task tracking, not live context measurement.

Specifically:

- Grite records decisions and discoveries as issues or memory entries.
- Grite gives future sessions a repo-native place to discover why a continuity rule exists.
- Grite does not know the live Codex prompt load for a thread.

So the split is:

- Codex session artifacts answer: "How full is this agent's latest prompt context?"
- Grite answers: "Why do we have this policy and what decisions have we already made?"

## Scripts

- [scripts/context_pool_watch.py](/Users/ali/brat_repo/scripts/context_pool_watch.py)
  - Polls thread telemetry and updates the pool state.
- [scripts/session_handoff.py](/Users/ali/brat_repo/scripts/session_handoff.py)
  - Builds a human-readable handoff file from session transcripts.
- [scripts/continuity_supervisor.py](/Users/ali/brat_repo/scripts/continuity_supervisor.py)
  - High-level daemon that combines pool telemetry, compaction detection, documentation companion output, and caretaker-slot enforcement.
- [scripts/start_continuity_supervisor.sh](/Users/ali/brat_repo/scripts/start_continuity_supervisor.sh)
  - Starts the continuity supervisor in a dedicated `tmux` session.
- [scripts/test_continuity_stack.sh](/Users/ali/brat_repo/scripts/test_continuity_stack.sh)
  - Runs the full continuity validation suite: unit tests, Python compile checks, one-shot artifact generation, and tmux launcher smoke test.

## Operating Model

1. Keep important threads under active watch.
2. Retire a thread from the default active pool once its prompt-load metric reaches `70%`.
3. Do not delete retired threads.
4. Use `session-handoff.md` plus the exact referenced session JSONL line numbers for recovery.
5. If a future session needs deeper detail, read the referenced JSONL transcript directly instead of inventing a summary.
6. For coding work, also read `.brat/continuity/project-continuity.md` to see which terminal sessions have touched the repo and which important docs or dotfiles should be re-reviewed.

## Documentation Companion

The continuity system now includes a daemon-backed documentation companion.

It is not a free-running language model thread. It is a persistent monitor that rewrites durable files on every poll so future terminals can recover without relying on memory.

Its required outputs are:

- `continuity-companion.md`
- `continuity-supervisor.json`
- `session-handoff.md`

The companion report explicitly states that it exists, where its artifacts live, and what the current continuity protocol is. That way a compacted or fresh terminal can discover the companion from disk instead of relying on prior conversation.

## Coding Continuity

Conversation continuity is not enough for coding work.

The continuity supervisor now also writes a repo-local project continuity report:

- `.brat/continuity/project-continuity.md`
- `.brat/continuity/project-continuity.json`

That project report enforces two extra rules:

- interaction log comes first,
- important review files come second.

### Terminal Interaction Log

Every terminal session that is actively working on the project must be logged with:

- terminal id,
- date and time,
- host/computer,
- project root,
- current working directory,
- main thread id,
- interaction count.

The same session record is updated on every poll, so staleness can be judged by interaction count instead of only elapsed wall-clock time.

### Important Files Inventory

The project report also maintains a list of review-worthy files, including:

- `AGENTS.md`
- architecture and design markdown files,
- repo docs under `docs/`,
- dotfiles and hidden tooling documents such as `.env.example`, `.claude/*.md`, and similar config or instruction files.

This gives future terminals a concrete checklist of files that should be re-reviewed before or during coding work.

## Caretaker Protocol

Main and supervisor threads are treated differently from subagents.

- Main or supervisor at/above threshold:
  - retire that thread from default reuse,
  - keep it on ice,
  - bring in a fresh replacement,
  - keep the handoff artifacts updated.
- Subagent at/above threshold:
  - keep monitoring it,
  - park it on retirement,
  - do not force a replacement unless needed for actual work.

Caretaker count is derived from the main thread's actual compaction count:

- first compaction => `1` caretaker slot required
- second or later compaction => `2` caretaker slots required
- cap at `2`

The current implementation uses daemon-backed caretaker slots that verify:

- documentation artifacts still exist,
- the main thread is still documented,
- the retirement policy is still encoded in the supervisor state.

## Testing

The continuity stack now has a single repeatable smoke entrypoint:

```bash
./scripts/test_continuity_stack.sh
```

That script validates all major elements:

- unit tests for compaction parsing, caretaker logic, project interaction logging, and report ordering,
- Python compile checks for the continuity scripts,
- one-shot `context_pool_watch.py`,
- one-shot `session_handoff.py`,
- one-shot `continuity_supervisor.py` with temp outputs,
- tmux launcher smoke test with an isolated log file.

If a future change touches the continuity stack, this script should be the first end-to-end check.

## Why This Is Better Than A Manual "Buddy"

This model is more robust because it:

- reads from actual Codex session transcripts,
- uses measured prompt-load telemetry,
- stores exact file references for recovery,
- avoids relying on a separate agent to remember everything correctly.

Manual summaries can still be useful, but they are secondary. The primary source of truth is the local Codex session log.
