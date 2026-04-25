#!/usr/bin/env python3
import argparse
import json
import os
import signal
import sqlite3
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from session_handoff import build_handoff, load_threads_from_db as load_handoff_threads, render_markdown


STATE_DB = Path.home() / ".codex" / "state_5.sqlite"
SESSIONS_DIR = Path.home() / ".codex" / "sessions"


@dataclass
class ThreadSnapshot:
    thread_id: str
    nickname: str | None
    title: str | None
    session_file: str | None
    last_event_at: str | None
    input_tokens: int | None
    cached_input_tokens: int | None
    output_tokens: int | None
    reasoning_output_tokens: int | None
    model_context_window: int | None
    pct_of_window: float | None
    pool_status: str


def load_threads_from_db() -> dict[str, dict[str, Any]]:
    if not STATE_DB.exists():
        return {}
    conn = sqlite3.connect(str(STATE_DB))
    try:
        rows = conn.execute(
            """
            select id, title, agent_nickname
            from threads
            """
        ).fetchall()
    finally:
        conn.close()
    return {
        row[0]: {"title": row[1], "nickname": row[2]}
        for row in rows
    }


def find_session_file(thread_id: str) -> Path | None:
    pattern = f"*{thread_id}.jsonl"
    matches = sorted(SESSIONS_DIR.glob(f"**/{pattern}"), reverse=True)
    return matches[0] if matches else None


def parse_latest_token_event(session_file: Path) -> tuple[str | None, dict[str, Any] | None, dict[str, Any] | None]:
    session_meta = None
    last_event = None
    last_event_at = None
    with session_file.open() as handle:
        for line in handle:
            obj = json.loads(line)
            if obj.get("type") == "session_meta":
                session_meta = obj.get("payload")
                continue
            payload = obj.get("payload", {})
            if obj.get("type") == "event_msg" and payload.get("type") == "token_count" and payload.get("info"):
                last_event = payload.get("info")
                last_event_at = obj.get("timestamp")
    return last_event_at, session_meta, last_event


def snapshot_thread(thread_id: str, threshold: float, db_threads: dict[str, dict[str, Any]]) -> ThreadSnapshot:
    db_info = db_threads.get(thread_id, {})
    session_file = find_session_file(thread_id)
    nickname = db_info.get("nickname")
    title = db_info.get("title")
    last_event_at = None
    input_tokens = None
    cached_input_tokens = None
    output_tokens = None
    reasoning_output_tokens = None
    model_context_window = None
    pct_of_window = None

    if session_file:
        last_event_at, session_meta, last_event = parse_latest_token_event(session_file)
        if session_meta and not nickname:
            nickname = session_meta.get("agent_nickname")
        if session_meta and not title:
            title = session_meta.get("title")
        if last_event:
            usage = last_event["last_token_usage"]
            input_tokens = usage.get("input_tokens")
            cached_input_tokens = usage.get("cached_input_tokens")
            output_tokens = usage.get("output_tokens")
            reasoning_output_tokens = usage.get("reasoning_output_tokens")
            model_context_window = last_event.get("model_context_window")
            if input_tokens is not None and model_context_window:
                pct_of_window = round((input_tokens / model_context_window) * 100, 2)

    if pct_of_window is None:
        pool_status = "unknown"
    elif pct_of_window >= threshold:
        pool_status = "retired"
    else:
        pool_status = "active"

    return ThreadSnapshot(
        thread_id=thread_id,
        nickname=nickname,
        title=title,
        session_file=str(session_file) if session_file else None,
        last_event_at=last_event_at,
        input_tokens=input_tokens,
        cached_input_tokens=cached_input_tokens,
        output_tokens=output_tokens,
        reasoning_output_tokens=reasoning_output_tokens,
        model_context_window=model_context_window,
        pct_of_window=pct_of_window,
        pool_status=pool_status,
    )


def snapshot_many(thread_ids: list[str], threshold: float) -> dict[str, Any]:
    db_threads = load_threads_from_db()
    snapshots = [snapshot_thread(thread_id, threshold, db_threads) for thread_id in thread_ids]
    active = [s.thread_id for s in snapshots if s.pool_status == "active"]
    retired = [s.thread_id for s in snapshots if s.pool_status == "retired"]
    unknown = [s.thread_id for s in snapshots if s.pool_status == "unknown"]
    return {
        "observed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "threshold_pct": threshold,
        "active_pool": active,
        "retired_pool": retired,
        "unknown_pool": unknown,
        "threads": [s.__dict__ for s in snapshots],
    }


def render(snapshot: dict[str, Any]) -> str:
    lines = []
    lines.append(
        f"[{snapshot['observed_at']}] threshold={snapshot['threshold_pct']}% "
        f"active={len(snapshot['active_pool'])} retired={len(snapshot['retired_pool'])} "
        f"unknown={len(snapshot['unknown_pool'])}"
    )
    for thread in snapshot["threads"]:
        pct = "n/a" if thread["pct_of_window"] is None else f"{thread['pct_of_window']:.2f}%"
        nick = thread["nickname"] or "-"
        title = (thread["title"] or "").replace("\n", " ")
        if len(title) > 52:
            title = title[:49] + "..."
        lines.append(
            f"{thread['pool_status']:>7}  {pct:>8}  {nick:<10}  {thread['thread_id']}  {title}"
        )
    return "\n".join(lines)


def write_snapshot(snapshot: dict[str, Any], state_file: Path, events_file: Path, previous: dict[str, str]) -> dict[str, str]:
    state_file.parent.mkdir(parents=True, exist_ok=True)
    events_file.parent.mkdir(parents=True, exist_ok=True)
    state_file.write_text(json.dumps(snapshot, indent=2) + "\n")
    current = {}
    with events_file.open("a") as handle:
        for thread in snapshot["threads"]:
            thread_id = thread["thread_id"]
            status = thread["pool_status"]
            current[thread_id] = status
            if previous.get(thread_id) != status:
                handle.write(json.dumps({
                    "observed_at": snapshot["observed_at"],
                    "thread_id": thread_id,
                    "nickname": thread["nickname"],
                    "from": previous.get(thread_id),
                    "to": status,
                    "pct_of_window": thread["pct_of_window"],
                }) + "\n")
    return current


def write_handoff(thread_ids: list[str], threshold: float, handoff_file: Path) -> None:
    db_threads = load_handoff_threads()
    handoffs = [build_handoff(thread_id, threshold, db_threads) for thread_id in thread_ids]
    handoff_file.parent.mkdir(parents=True, exist_ok=True)
    handoff_file.write_text(render_markdown(handoffs, threshold))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--thread", action="append", dest="threads", default=[])
    parser.add_argument("--interval", type=int, default=30)
    parser.add_argument("--threshold", type=float, default=70.0)
    parser.add_argument("--once", action="store_true")
    parser.add_argument("--state-file", default=str(Path.home() / ".codex" / "context-pool.json"))
    parser.add_argument("--events-file", default=str(Path.home() / ".codex" / "context-pool-events.jsonl"))
    parser.add_argument("--handoff-file", default=str(Path.home() / ".codex" / "session-handoff.md"))
    args = parser.parse_args()

    thread_ids = [t for t in args.threads if t]
    if not thread_ids:
        print("No threads specified.", file=sys.stderr)
        return 1

    stop = False

    def handle_signal(_signum: int, _frame: Any) -> None:
        nonlocal stop
        stop = True

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    previous: dict[str, str] = {}
    state_file = Path(args.state_file)
    events_file = Path(args.events_file)
    handoff_file = Path(args.handoff_file)

    while not stop:
        snapshot = snapshot_many(thread_ids, args.threshold)
        previous = write_snapshot(snapshot, state_file, events_file, previous)
        write_handoff(thread_ids, args.threshold, handoff_file)
        print(render(snapshot), flush=True)
        if args.once:
            break
        time.sleep(args.interval)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
