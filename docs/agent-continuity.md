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

## Operating Model

1. Keep important threads under active watch.
2. Retire a thread from the default active pool once its prompt-load metric reaches `70%`.
3. Do not delete retired threads.
4. Use `session-handoff.md` plus the exact referenced session JSONL line numbers for recovery.
5. If a future session needs deeper detail, read the referenced JSONL transcript directly instead of inventing a summary.

## Why This Is Better Than A Manual "Buddy"

This model is more robust because it:

- reads from actual Codex session transcripts,
- uses measured prompt-load telemetry,
- stores exact file references for recovery,
- avoids relying on a separate agent to remember everything correctly.

Manual summaries can still be useful, but they are secondary. The primary source of truth is the local Codex session log.
