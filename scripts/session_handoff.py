#!/usr/bin/env python3
import argparse
import json
import sqlite3
from dataclasses import dataclass
from pathlib import Path
from typing import Any


STATE_DB = Path.home() / ".codex" / "state_5.sqlite"
SESSIONS_DIR = Path.home() / ".codex" / "sessions"


@dataclass
class MessageRef:
    timestamp: str
    line_no: int
    text: str


@dataclass
class ThreadHandoff:
    thread_id: str
    nickname: str | None
    title: str | None
    session_file: str | None
    last_event_at: str | None
    prompt_pct: float | None
    input_tokens: int | None
    cached_input_tokens: int | None
    model_context_window: int | None
    last_user_messages: list[MessageRef]
    pool_status: str


def load_threads_from_db() -> dict[str, dict[str, Any]]:
    if not STATE_DB.exists():
        return {}
    conn = sqlite3.connect(str(STATE_DB))
    try:
        rows = conn.execute("select id, title, agent_nickname from threads").fetchall()
    finally:
        conn.close()
    return {row[0]: {"title": row[1], "nickname": row[2]} for row in rows}


def find_session_file(thread_id: str) -> Path | None:
    matches = sorted(SESSIONS_DIR.glob(f"**/*{thread_id}.jsonl"), reverse=True)
    return matches[0] if matches else None


def parse_session_file(session_file: Path) -> tuple[str | None, dict[str, Any] | None, dict[str, Any] | None, list[MessageRef]]:
    session_meta = None
    last_token = None
    last_event_at = None
    user_messages: list[MessageRef] = []
    with session_file.open() as handle:
        for line_no, line in enumerate(handle, start=1):
            obj = json.loads(line)
            if obj.get("type") == "session_meta":
                session_meta = obj.get("payload")
            payload = obj.get("payload", {})
            if obj.get("type") == "event_msg" and payload.get("type") == "token_count" and payload.get("info"):
                last_token = payload.get("info")
                last_event_at = obj.get("timestamp")
            if obj.get("type") == "event_msg" and payload.get("type") == "user_message":
                text = (payload.get("message") or "").strip().replace("\n", " ")
                user_messages.append(MessageRef(
                    timestamp=obj.get("timestamp", ""),
                    line_no=line_no,
                    text=text,
                ))
    return last_event_at, session_meta, last_token, user_messages[-3:]


def build_handoff(thread_id: str, threshold: float, db_threads: dict[str, dict[str, Any]]) -> ThreadHandoff:
    db_info = db_threads.get(thread_id, {})
    session_file = find_session_file(thread_id)
    nickname = db_info.get("nickname")
    title = db_info.get("title")
    last_event_at = None
    prompt_pct = None
    input_tokens = None
    cached_input_tokens = None
    model_context_window = None
    last_user_messages: list[MessageRef] = []
    if session_file:
        last_event_at, session_meta, last_token, last_user_messages = parse_session_file(session_file)
        if session_meta and not nickname:
            nickname = session_meta.get("agent_nickname")
        if session_meta and not title:
            title = session_meta.get("title")
        if last_token:
            last_usage = last_token["last_token_usage"]
            input_tokens = last_usage.get("input_tokens")
            cached_input_tokens = last_usage.get("cached_input_tokens")
            model_context_window = last_token.get("model_context_window")
            if input_tokens is not None and model_context_window:
                prompt_pct = round((input_tokens / model_context_window) * 100, 2)
    if prompt_pct is None:
        pool_status = "unknown"
    elif prompt_pct >= threshold:
        pool_status = "retired"
    else:
        pool_status = "active"
    return ThreadHandoff(
        thread_id=thread_id,
        nickname=nickname,
        title=title,
        session_file=str(session_file) if session_file else None,
        last_event_at=last_event_at,
        prompt_pct=prompt_pct,
        input_tokens=input_tokens,
        cached_input_tokens=cached_input_tokens,
        model_context_window=model_context_window,
        last_user_messages=last_user_messages,
        pool_status=pool_status,
    )


def render_markdown(handoffs: list[ThreadHandoff], threshold: float) -> str:
    lines: list[str] = []
    lines.append("# Session Handoff")
    lines.append("")
    lines.append("This file is generated from local Codex session artifacts.")
    lines.append(f"Retirement threshold: `{threshold:.1f}%` of last prompt input tokens vs model context window.")
    lines.append("")
    for handoff in handoffs:
        lines.append(f"## {handoff.nickname or 'Main'}")
        lines.append("")
        lines.append(f"- `thread_id`: `{handoff.thread_id}`")
        lines.append(f"- `status`: `{handoff.pool_status}`")
        if handoff.prompt_pct is not None:
            lines.append(
                f"- `prompt_load`: `{handoff.prompt_pct:.2f}%` "
                f"({handoff.input_tokens}/{handoff.model_context_window}, cached {handoff.cached_input_tokens})"
            )
        else:
            lines.append("- `prompt_load`: `unknown`")
        if handoff.last_event_at:
            lines.append(f"- `last_event_at`: `{handoff.last_event_at}`")
        if handoff.title:
            lines.append(f"- `title`: {handoff.title}")
        if handoff.session_file:
            lines.append(f"- `session_file`: `{handoff.session_file}`")
        lines.append("- `recent_user_messages`:")
        if handoff.last_user_messages:
            for ref in handoff.last_user_messages:
                text = ref.text if len(ref.text) <= 140 else ref.text[:137] + "..."
                lines.append(
                    f"  - `{ref.timestamp}` line `{ref.line_no}`: {text}"
                )
        else:
            lines.append("  - none")
        lines.append("")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--thread", action="append", dest="threads", default=[])
    parser.add_argument("--threshold", type=float, default=70.0)
    parser.add_argument("--output", default=str(Path.home() / ".codex" / "session-handoff.md"))
    args = parser.parse_args()
    if not args.threads:
        raise SystemExit("No threads specified.")
    db_threads = load_threads_from_db()
    handoffs = [build_handoff(thread_id, args.threshold, db_threads) for thread_id in args.threads]
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(render_markdown(handoffs, args.threshold))
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
