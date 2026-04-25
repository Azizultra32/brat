#!/usr/bin/env python3
import argparse
import json
import os
import signal
import socket
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from context_pool_watch import (
    find_session_file,
    load_threads_from_db,
    render as render_pool_snapshot,
    snapshot_many,
    write_handoff,
    write_snapshot,
)


@dataclass
class CompactionRef:
    timestamp: str
    line_no: int
    message_preview: str


IMPORTANT_DOC_NAMES = {
    "agents.md",
    "architecture.md",
    "readme.md",
    "contributing.md",
    "roadmap.md",
    "operations.md",
    "daemon.md",
    "agent-playbook.md",
    "cli.md",
    "design.md",
    "spec.md",
    "canonical-spec.md",
    "data-model.md",
    "roles.md",
    "actors.md",
    "merge-policy.md",
    "workflow.md",
    "workflows.md",
}
IMPORTANT_DOTFILE_NAMES = {
    ".env",
    ".env.example",
    ".env.local",
    ".envrc",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".nvmrc",
    ".python-version",
    ".tool-versions",
}
IMPORTANT_SUFFIXES = {".md", ".toml", ".json", ".yaml", ".yml"}
IGNORED_DIRS = {
    ".git",
    ".brat",
    ".grite",
    ".gritee",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    "__pycache__",
}


def read_terminal_id() -> str | None:
    tid_file = Path.home() / ".codex" / ".tid_current"
    if not tid_file.exists():
        return None
    value = tid_file.read_text().strip()
    return value or None


def scan_important_files(project_root: Path, limit: int) -> list[dict[str, Any]]:
    matches: list[dict[str, Any]] = []
    for path in project_root.rglob("*"):
        if not path.is_file():
            continue
        if any(part in IGNORED_DIRS for part in path.relative_to(project_root).parts[:-1]):
            continue

        relative_path = path.relative_to(project_root).as_posix()
        name_lower = path.name.lower()
        reasons: list[str] = []
        score = 0

        if name_lower == "agents.md":
            reasons.append("agent_instructions")
            score += 100
        if name_lower in IMPORTANT_DOC_NAMES:
            reasons.append("named_review_doc")
            score += 60
        if relative_path.startswith("docs/") and path.suffix.lower() == ".md":
            reasons.append("docs_markdown")
            score += 35
        if any(part.startswith(".") for part in path.relative_to(project_root).parts[:-1]) and path.suffix.lower() in IMPORTANT_SUFFIXES:
            reasons.append("hidden_tooling_doc")
            score += 30
        if path.name in IMPORTANT_DOTFILE_NAMES:
            reasons.append("dotfile")
            score += 45
        if path.name.startswith(".") and path.suffix.lower() in IMPORTANT_SUFFIXES:
            reasons.append("dotfile")
            score += 25
        if not reasons and path.suffix.lower() == ".md":
            reasons.append("markdown")
            score += 15

        if not reasons:
            continue

        stat = path.stat()
        matches.append({
            "path": relative_path,
            "reasons": reasons,
            "score": score,
            "mtime": stat.st_mtime,
            "size_bytes": stat.st_size,
        })

    matches.sort(key=lambda item: (-item["score"], item["path"]))
    selected = matches[:limit]
    for item in selected:
        item["mtime_iso"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(item["mtime"]))
        del item["mtime"]
    return selected


def load_json_file(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    return json.loads(path.read_text())


def update_project_continuity(
    project_root: Path,
    project_state_file: Path,
    project_report_file: Path,
    terminal_id: str | None,
    terminal_host: str,
    project_session_id: str,
    cwd: Path,
    observed_at: str,
    important_files_limit: int,
    main_thread_id: str,
) -> dict[str, Any]:
    state = load_json_file(project_state_file)
    sessions = state.get("sessions", [])
    session = next((item for item in sessions if item.get("project_session_id") == project_session_id), None)
    if session is None:
        session = {
            "project_session_id": project_session_id,
            "terminal_id": terminal_id,
            "host": terminal_host,
            "project_root": str(project_root),
            "cwd": str(cwd),
            "first_seen_at": observed_at,
            "last_seen_at": observed_at,
            "interaction_count": 1,
            "main_thread_id": main_thread_id,
        }
        sessions.append(session)
    else:
        session["last_seen_at"] = observed_at
        session["interaction_count"] = int(session.get("interaction_count", 0)) + 1
        session["cwd"] = str(cwd)
        session["main_thread_id"] = main_thread_id

    sessions.sort(key=lambda item: (item.get("last_seen_at") or "", item.get("first_seen_at") or ""), reverse=True)
    state.update({
        "observed_at": observed_at,
        "project_root": str(project_root),
        "current_session_id": project_session_id,
        "sessions": sessions[:200],
        "important_files": scan_important_files(project_root, important_files_limit),
    })

    project_state_file.parent.mkdir(parents=True, exist_ok=True)
    project_report_file.parent.mkdir(parents=True, exist_ok=True)
    project_state_file.write_text(json.dumps(state, indent=2) + "\n")
    project_report_file.write_text(render_project_continuity_markdown(state))
    return state


def render_project_continuity_markdown(state: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Project Continuity")
    lines.append("")
    lines.append("## Terminal Interaction Log")
    lines.append("")
    lines.append(
        "This section comes first on purpose. Every terminal session that works on this project must be logged here so staleness can be measured by interactions, not just elapsed time."
    )
    lines.append("")
    lines.append(f"- `observed_at`: `{state.get('observed_at')}`")
    lines.append(f"- `project_root`: `{state.get('project_root')}`")
    lines.append(f"- `current_session_id`: `{state.get('current_session_id')}`")
    lines.append("")
    for session in state.get("sessions", []):
        lines.append(f"### {session.get('project_session_id')}")
        lines.append("")
        lines.append(f"- `terminal_id`: `{session.get('terminal_id') or 'unknown'}`")
        lines.append(f"- `host`: `{session.get('host')}`")
        lines.append(f"- `project_root`: `{session.get('project_root')}`")
        lines.append(f"- `cwd`: `{session.get('cwd')}`")
        lines.append(f"- `first_seen_at`: `{session.get('first_seen_at')}`")
        lines.append(f"- `last_seen_at`: `{session.get('last_seen_at')}`")
        lines.append(f"- `interaction_count`: `{session.get('interaction_count')}`")
        lines.append(f"- `main_thread_id`: `{session.get('main_thread_id')}`")
        lines.append("")
    lines.append("## Important Files To Review")
    lines.append("")
    lines.append(
        "These are the dotfiles, agent files, architecture docs, and other project documents that should be reviewed regularly during coding work."
    )
    lines.append("")
    for item in state.get("important_files", []):
        reasons = ", ".join(item["reasons"])
        lines.append(f"- `{item['path']}`")
        lines.append(f"  reasons: `{reasons}`")
        lines.append(f"  last_modified: `{item['mtime_iso']}`")
        lines.append(f"  size_bytes: `{item['size_bytes']}`")
    lines.append("")
    return "\n".join(lines)


def parse_compaction_events(session_file: Path) -> list[CompactionRef]:
    events: list[CompactionRef] = []
    with session_file.open() as handle:
        for line_no, line in enumerate(handle, start=1):
            obj = json.loads(line)
            if obj.get("type") != "compacted":
                continue
            payload = obj.get("payload", {})
            message = payload.get("message") or ""
            message = message.strip().replace("\n", " ")
            if len(message) > 180:
                message = message[:177] + "..."
            events.append(CompactionRef(
                timestamp=obj.get("timestamp", ""),
                line_no=line_no,
                message_preview=message,
            ))
    return events


def build_compaction_index(thread_ids: list[str], db_threads: dict[str, dict[str, Any]]) -> dict[str, dict[str, Any]]:
    index: dict[str, dict[str, Any]] = {}
    for thread_id in thread_ids:
        db_info = db_threads.get(thread_id, {})
        session_file = find_session_file(thread_id)
        events: list[CompactionRef] = []
        if session_file:
            events = parse_compaction_events(session_file)
        index[thread_id] = {
            "thread_id": thread_id,
            "nickname": db_info.get("nickname"),
            "title": db_info.get("title"),
            "session_file": str(session_file) if session_file else None,
            "compaction_count": len(events),
            "last_compaction_at": events[-1].timestamp if events else None,
            "recent_compactions": [event.__dict__ for event in events[-3:]],
        }
    return index


def build_caretaker_slots(
    required_count: int,
    companion_file: Path,
    handoff_file: Path,
    main_thread_state: dict[str, Any],
    observed_at: str,
) -> list[dict[str, Any]]:
    slots: list[dict[str, Any]] = []
    documented = bool(main_thread_state.get("thread_id"))
    rotation_ok = main_thread_state.get("retirement_action") == "rotate_on_retirement"

    for slot_no in range(1, 3):
        enabled = slot_no <= required_count
        checks = [
            {
                "name": "documentation_present",
                "ok": companion_file.exists(),
                "detail": str(companion_file),
            },
            {
                "name": "handoff_present",
                "ok": handoff_file.exists(),
                "detail": str(handoff_file),
            },
            {
                "name": "main_thread_documented",
                "ok": documented,
                "detail": main_thread_state.get("thread_id"),
            },
            {
                "name": "retirement_policy_documented",
                "ok": rotation_ok,
                "detail": main_thread_state.get("retirement_action"),
            },
        ]
        slot_status = "active" if enabled else "standby"
        if enabled and not all(check["ok"] for check in checks):
            slot_status = "degraded"
        slots.append({
            "slot": slot_no,
            "kind": "daemon-backed caretaker",
            "status": slot_status,
            "enabled": enabled,
            "activated_at": observed_at if enabled else None,
            "checks": checks,
            "purpose": (
                "Questions whether documentation and retirement rules stayed intact after compaction."
            ),
        })
    return slots


def build_supervisor_state(
    pool_snapshot: dict[str, Any],
    compaction_index: dict[str, dict[str, Any]],
    main_thread_id: str,
    supervisor_threads: set[str],
    worker_threads: set[str],
    threshold: float,
    artifact_paths: dict[str, Path],
) -> dict[str, Any]:
    observed_at = pool_snapshot["observed_at"]
    threads: list[dict[str, Any]] = []
    pool_threads = {thread["thread_id"]: thread for thread in pool_snapshot["threads"]}

    for thread_id, thread in pool_threads.items():
        if thread_id == main_thread_id:
            role = "main"
        elif thread_id in supervisor_threads:
            role = "supervisor"
        elif thread_id in worker_threads:
            role = "subagent"
        else:
            role = "monitored"

        retirement_action = (
            "rotate_on_retirement" if role in {"main", "supervisor"} else "park_on_retirement"
        )
        merged = {
            **thread,
            "role": role,
            "retirement_action": retirement_action,
            **compaction_index.get(thread_id, {}),
        }
        threads.append(merged)

    threads.sort(key=lambda item: (0 if item["role"] == "main" else 1, item["role"], item["thread_id"]))
    main_thread_state = next(thread for thread in threads if thread["thread_id"] == main_thread_id)
    main_compactions = main_thread_state.get("compaction_count", 0)
    required_caretakers = min(main_compactions, 2)

    documentation_companion = {
        "kind": "daemon-backed documentation companion",
        "status": "active",
        "verified_at": observed_at,
        "artifacts": {
            "pool_snapshot": str(artifact_paths["pool_snapshot"]),
            "pool_events": str(artifact_paths["pool_events"]),
            "handoff": str(artifact_paths["handoff"]),
            "supervisor_state": str(artifact_paths["supervisor_state"]),
            "companion_report": str(artifact_paths["companion_report"]),
        },
        "purpose": (
            "Persist the continuity protocol, current prompt-load state, compaction history, "
            "and exact transcript references for future terminals."
        ),
        "notes": [
            "Any compacted or fresh terminal should read the companion report and session handoff first.",
            "This companion is persistent because the daemon rewrites its outputs on every poll.",
            "It explicitly documents that the companion exists so recovery does not depend on memory.",
        ],
    }

    caretaker_slots = build_caretaker_slots(
        required_caretakers,
        companion_file=artifact_paths["companion_report"],
        handoff_file=artifact_paths["handoff"],
        main_thread_state=main_thread_state,
        observed_at=observed_at,
    )

    return {
        "observed_at": observed_at,
        "threshold_pct": threshold,
        "main_thread_id": main_thread_id,
        "main_thread_prompt_load_pct": main_thread_state.get("pct_of_window"),
        "main_thread_pool_status": main_thread_state.get("pool_status"),
        "main_thread_compactions": main_compactions,
        "required_caretakers": required_caretakers,
        "documentation_companion": documentation_companion,
        "caretaker_slots": caretaker_slots,
        "threads": threads,
    }


def render_supervisor_markdown(state: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Continuity Companion")
    lines.append("")
    lines.append("This file is generated from local Codex session artifacts.")
    lines.append("")
    lines.append("## Protocol")
    lines.append("")
    lines.append(
        f"- Main and supervisor threads retire at `{state['threshold_pct']:.1f}%` prompt load."
    )
    lines.append("- Retired main/supervisor threads stay on ice and require a fresh replacement thread.")
    lines.append("- Subagents are still monitored, but they park on retirement instead of forcing rotation.")
    lines.append("- The documentation companion must exist in this report and in the JSON supervisor state.")
    lines.append("- Required caretaker count equals main-thread compactions, capped at `2`.")
    lines.append("")
    lines.append("## Current Status")
    lines.append("")
    lines.append(f"- `observed_at`: `{state['observed_at']}`")
    lines.append(f"- `main_thread_id`: `{state['main_thread_id']}`")
    lines.append(f"- `main_prompt_load`: `{state['main_thread_prompt_load_pct']:.2f}%`")
    lines.append(f"- `main_pool_status`: `{state['main_thread_pool_status']}`")
    lines.append(f"- `main_compactions`: `{state['main_thread_compactions']}`")
    lines.append(f"- `required_caretakers`: `{state['required_caretakers']}`")
    lines.append("")
    lines.append("## Documentation Companion")
    lines.append("")
    companion = state["documentation_companion"]
    lines.append(f"- `status`: `{companion['status']}`")
    lines.append(f"- `verified_at`: `{companion['verified_at']}`")
    lines.append(f"- `kind`: `{companion['kind']}`")
    lines.append("- `artifacts`:")
    for key, value in companion["artifacts"].items():
        lines.append(f"  - `{key}`: `{value}`")
    lines.append("- `notes`:")
    for note in companion["notes"]:
        lines.append(f"  - {note}")
    lines.append("")
    lines.append("## Caretaker Slots")
    lines.append("")
    for slot in state["caretaker_slots"]:
        lines.append(f"### Caretaker {slot['slot']}")
        lines.append("")
        lines.append(f"- `status`: `{slot['status']}`")
        lines.append(f"- `enabled`: `{slot['enabled']}`")
        lines.append(f"- `kind`: `{slot['kind']}`")
        lines.append(f"- `purpose`: {slot['purpose']}")
        lines.append("- `checks`:")
        for check in slot["checks"]:
            flag = "PASS" if check["ok"] else "FAIL"
            lines.append(f"  - `{flag}` {check['name']}: `{check['detail']}`")
        lines.append("")
    lines.append("## Monitored Threads")
    lines.append("")
    for thread in state["threads"]:
        pct = "unknown" if thread["pct_of_window"] is None else f"{thread['pct_of_window']:.2f}%"
        lines.append(f"### {thread.get('nickname') or thread['role'].title()}")
        lines.append("")
        lines.append(f"- `thread_id`: `{thread['thread_id']}`")
        lines.append(f"- `role`: `{thread['role']}`")
        lines.append(f"- `status`: `{thread['pool_status']}`")
        lines.append(f"- `prompt_load`: `{pct}`")
        lines.append(f"- `retirement_action`: `{thread['retirement_action']}`")
        lines.append(f"- `compactions`: `{thread.get('compaction_count', 0)}`")
        if thread.get("last_compaction_at"):
            lines.append(f"- `last_compaction_at`: `{thread['last_compaction_at']}`")
        if thread.get("session_file"):
            lines.append(f"- `session_file`: `{thread['session_file']}`")
        recent = thread.get("recent_compactions") or []
        lines.append("- `recent_compactions`:")
        if recent:
            for event in recent:
                detail = event["message_preview"] or "(empty summary)"
                lines.append(
                    f"  - `{event['timestamp']}` line `{event['line_no']}`: {detail}"
                )
        else:
            lines.append("  - none")
        lines.append("")
    lines.append("## Recovery")
    lines.append("")
    lines.append("- Read `~/.codex/continuity-supervisor.json` for machine-readable state.")
    lines.append("- Read `~/.codex/session-handoff.md` for exact recent user-message references.")
    lines.append("- Read the referenced session JSONL transcripts directly if deeper detail is required.")
    return "\n".join(lines) + "\n"


def render_supervisor_summary(state: dict[str, Any], pool_snapshot: dict[str, Any]) -> str:
    lines = [
        render_pool_snapshot(pool_snapshot),
        "",
        (
            f"main={state['main_thread_id']} load={state['main_thread_prompt_load_pct']:.2f}% "
            f"compactions={state['main_thread_compactions']} "
            f"caretakers={state['required_caretakers']} "
            f"doc_companion={state['documentation_companion']['status']}"
        ),
    ]
    return "\n".join(lines)


def write_supervisor_outputs(
    state: dict[str, Any],
    supervisor_file: Path,
    companion_file: Path,
    events_file: Path,
    previous: dict[str, Any],
) -> dict[str, Any]:
    supervisor_file.parent.mkdir(parents=True, exist_ok=True)
    companion_file.parent.mkdir(parents=True, exist_ok=True)
    events_file.parent.mkdir(parents=True, exist_ok=True)

    supervisor_file.write_text(json.dumps(state, indent=2) + "\n")
    companion_file.write_text(render_supervisor_markdown(state))

    current = {
        "main_thread_compactions": state["main_thread_compactions"],
        "required_caretakers": state["required_caretakers"],
        "main_thread_pool_status": state["main_thread_pool_status"],
    }
    with events_file.open("a") as handle:
        for key, value in current.items():
            if previous.get(key) != value:
                handle.write(json.dumps({
                    "observed_at": state["observed_at"],
                    "key": key,
                    "from": previous.get(key),
                    "to": value,
                }) + "\n")
    return current


def unique_threads(*groups: list[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for group in groups:
        for item in group:
            if not item or item in seen:
                continue
            seen.add(item)
            result.append(item)
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--main-thread", required=True)
    parser.add_argument("--supervisor-thread", action="append", default=[])
    parser.add_argument("--worker-thread", action="append", default=[])
    parser.add_argument("--thread", action="append", default=[])
    parser.add_argument("--interval", type=int, default=30)
    parser.add_argument("--threshold", type=float, default=70.0)
    parser.add_argument("--once", action="store_true")
    parser.add_argument("--state-file", default=str(Path.home() / ".codex" / "context-pool.json"))
    parser.add_argument("--events-file", default=str(Path.home() / ".codex" / "context-pool-events.jsonl"))
    parser.add_argument("--handoff-file", default=str(Path.home() / ".codex" / "session-handoff.md"))
    parser.add_argument(
        "--supervisor-file",
        default=str(Path.home() / ".codex" / "continuity-supervisor.json"),
    )
    parser.add_argument(
        "--companion-file",
        default=str(Path.home() / ".codex" / "continuity-companion.md"),
    )
    parser.add_argument(
        "--supervisor-events-file",
        default=str(Path.home() / ".codex" / "continuity-supervisor-events.jsonl"),
    )
    parser.add_argument("--project-root", default=os.getcwd())
    parser.add_argument("--project-state-file")
    parser.add_argument("--project-report-file")
    parser.add_argument("--terminal-id")
    parser.add_argument("--terminal-host")
    parser.add_argument("--project-session-id")
    parser.add_argument("--cwd", default=os.getcwd())
    parser.add_argument("--important-files-limit", type=int, default=80)
    args = parser.parse_args()

    thread_ids = unique_threads(
        [args.main_thread],
        args.supervisor_thread,
        args.worker_thread,
        args.thread,
    )

    stop = False

    def handle_signal(_signum: int, _frame: Any) -> None:
        nonlocal stop
        stop = True

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    previous_pool: dict[str, str] = {}
    previous_supervisor: dict[str, Any] = {}

    state_file = Path(args.state_file)
    events_file = Path(args.events_file)
    handoff_file = Path(args.handoff_file)
    supervisor_file = Path(args.supervisor_file)
    companion_file = Path(args.companion_file)
    supervisor_events_file = Path(args.supervisor_events_file)
    project_root = Path(args.project_root).resolve()
    project_state_file = (
        Path(args.project_state_file)
        if args.project_state_file
        else project_root / ".brat" / "continuity" / "project-continuity.json"
    )
    project_report_file = (
        Path(args.project_report_file)
        if args.project_report_file
        else project_root / ".brat" / "continuity" / "project-continuity.md"
    )
    terminal_id = args.terminal_id or read_terminal_id()
    terminal_host = args.terminal_host or socket.gethostname()
    project_session_id = (
        args.project_session_id
        or f"{terminal_id or 'unknown-terminal'}-{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())}"
    )
    current_cwd = Path(args.cwd).resolve()
    artifact_paths = {
        "pool_snapshot": state_file,
        "pool_events": events_file,
        "handoff": handoff_file,
        "supervisor_state": supervisor_file,
        "companion_report": companion_file,
    }

    while not stop:
        pool_snapshot = snapshot_many(thread_ids, args.threshold)
        previous_pool = write_snapshot(pool_snapshot, state_file, events_file, previous_pool)
        write_handoff(thread_ids, args.threshold, handoff_file)
        companion_file.parent.mkdir(parents=True, exist_ok=True)
        companion_file.touch(exist_ok=True)

        db_threads = load_threads_from_db()
        compaction_index = build_compaction_index(thread_ids, db_threads)
        supervisor_state = build_supervisor_state(
            pool_snapshot=pool_snapshot,
            compaction_index=compaction_index,
            main_thread_id=args.main_thread,
            supervisor_threads=set(args.supervisor_thread),
            worker_threads=set(args.worker_thread),
            threshold=args.threshold,
            artifact_paths=artifact_paths,
        )
        previous_supervisor = write_supervisor_outputs(
            supervisor_state,
            supervisor_file=supervisor_file,
            companion_file=companion_file,
            events_file=supervisor_events_file,
            previous=previous_supervisor,
        )
        update_project_continuity(
            project_root=project_root,
            project_state_file=project_state_file,
            project_report_file=project_report_file,
            terminal_id=terminal_id,
            terminal_host=terminal_host,
            project_session_id=project_session_id,
            cwd=current_cwd,
            observed_at=pool_snapshot["observed_at"],
            important_files_limit=args.important_files_limit,
            main_thread_id=args.main_thread,
        )

        print(render_supervisor_summary(supervisor_state, pool_snapshot), flush=True)
        if args.once:
            break
        time.sleep(args.interval)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
