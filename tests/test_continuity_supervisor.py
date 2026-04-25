import json
import tempfile
import unittest
from pathlib import Path

import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))

from continuity_supervisor import (  # noqa: E402
    build_caretaker_slots,
    build_supervisor_state,
    parse_compaction_events,
    scan_important_files,
    update_project_continuity,
)


class ContinuitySupervisorTests(unittest.TestCase):
    def test_parse_compaction_events_only_counts_compacted_records(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            session_file = Path(tmpdir) / "session.jsonl"
            records = [
                {"timestamp": "2026-04-24T00:00:00Z", "type": "session_meta", "payload": {}},
                {
                    "timestamp": "2026-04-24T00:01:00Z",
                    "type": "compacted",
                    "payload": {"message": "First compacted summary"},
                },
                {
                    "timestamp": "2026-04-24T00:01:01Z",
                    "type": "event_msg",
                    "payload": {"type": "context_compacted"},
                },
                {
                    "timestamp": "2026-04-24T00:02:00Z",
                    "type": "compacted",
                    "payload": {"message": "Second compacted summary"},
                },
            ]
            session_file.write_text("".join(json.dumps(record) + "\n" for record in records))

            events = parse_compaction_events(session_file)

            self.assertEqual(2, len(events))
            self.assertEqual("2026-04-24T00:01:00Z", events[0].timestamp)
            self.assertEqual("Second compacted summary", events[1].message_preview)

    def test_build_supervisor_state_caps_caretakers_at_two(self) -> None:
        pool_snapshot = {
            "observed_at": "2026-04-25T01:00:00Z",
            "threshold_pct": 70.0,
            "active_pool": ["main-thread", "worker-thread"],
            "retired_pool": [],
            "unknown_pool": [],
            "threads": [
                {
                    "thread_id": "main-thread",
                    "nickname": None,
                    "title": "main",
                    "session_file": "/tmp/main.jsonl",
                    "last_event_at": "2026-04-25T00:59:00Z",
                    "input_tokens": 70000,
                    "cached_input_tokens": 1000,
                    "output_tokens": 500,
                    "reasoning_output_tokens": 100,
                    "model_context_window": 100000,
                    "pct_of_window": 70.0,
                    "pool_status": "retired",
                },
                {
                    "thread_id": "worker-thread",
                    "nickname": "Worker",
                    "title": "worker",
                    "session_file": "/tmp/worker.jsonl",
                    "last_event_at": "2026-04-25T00:59:00Z",
                    "input_tokens": 20000,
                    "cached_input_tokens": 1000,
                    "output_tokens": 500,
                    "reasoning_output_tokens": 100,
                    "model_context_window": 100000,
                    "pct_of_window": 20.0,
                    "pool_status": "active",
                },
            ],
        }
        compaction_index = {
            "main-thread": {
                "thread_id": "main-thread",
                "nickname": None,
                "title": "main",
                "session_file": "/tmp/main.jsonl",
                "compaction_count": 7,
                "last_compaction_at": "2026-04-25T00:58:00Z",
                "recent_compactions": [],
            },
            "worker-thread": {
                "thread_id": "worker-thread",
                "nickname": "Worker",
                "title": "worker",
                "session_file": "/tmp/worker.jsonl",
                "compaction_count": 0,
                "last_compaction_at": None,
                "recent_compactions": [],
            },
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            handoff_file = Path(tmpdir) / "session-handoff.md"
            companion_file = Path(tmpdir) / "continuity-companion.md"
            handoff_file.write_text("handoff")
            companion_file.write_text("companion")

            state = build_supervisor_state(
                pool_snapshot=pool_snapshot,
                compaction_index=compaction_index,
                main_thread_id="main-thread",
                supervisor_threads=set(),
                worker_threads={"worker-thread"},
                threshold=70.0,
                artifact_paths={
                    "pool_snapshot": Path(tmpdir) / "context-pool.json",
                    "pool_events": Path(tmpdir) / "context-pool-events.jsonl",
                    "handoff": handoff_file,
                    "supervisor_state": Path(tmpdir) / "continuity-supervisor.json",
                    "companion_report": companion_file,
                },
            )

            self.assertEqual(7, state["main_thread_compactions"])
            self.assertEqual(2, state["required_caretakers"])
            self.assertEqual("rotate_on_retirement", state["threads"][0]["retirement_action"])
            self.assertEqual("park_on_retirement", state["threads"][1]["retirement_action"])
            self.assertEqual("active", state["caretaker_slots"][0]["status"])
            self.assertEqual("active", state["caretaker_slots"][1]["status"])

    def test_build_caretaker_slots_fails_when_docs_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            missing_companion = Path(tmpdir) / "missing.md"
            missing_handoff = Path(tmpdir) / "missing-handoff.md"

            slots = build_caretaker_slots(
                required_count=1,
                companion_file=missing_companion,
                handoff_file=missing_handoff,
                main_thread_state={
                    "thread_id": "main-thread",
                    "retirement_action": "rotate_on_retirement",
                },
                observed_at="2026-04-25T01:00:00Z",
            )

            self.assertEqual("degraded", slots[0]["status"])
            self.assertEqual("standby", slots[1]["status"])
            self.assertFalse(slots[0]["checks"][0]["ok"])
            self.assertFalse(slots[0]["checks"][1]["ok"])

    def test_scan_important_files_prioritizes_agents_and_architecture_docs(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "AGENTS.md").write_text("agents")
            (root / "docs").mkdir()
            (root / "docs" / "architecture.md").write_text("architecture")
            (root / ".claude").mkdir()
            (root / ".claude" / "mayor_context.md").write_text("context")
            (root / ".env.example").write_text("X=1")
            (root / "target").mkdir()
            (root / "target" / "ignored.md").write_text("ignored")

            files = scan_important_files(root, limit=10)
            paths = [item["path"] for item in files]

            self.assertIn("AGENTS.md", paths)
            self.assertIn("docs/architecture.md", paths)
            self.assertIn(".claude/mayor_context.md", paths)
            self.assertIn(".env.example", paths)
            self.assertNotIn("target/ignored.md", paths)
            self.assertEqual("AGENTS.md", files[0]["path"])

    def test_update_project_continuity_tracks_terminal_interactions(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "AGENTS.md").write_text("agents")
            project_state_file = root / ".brat" / "continuity" / "project-continuity.json"
            project_report_file = root / ".brat" / "continuity" / "project-continuity.md"

            first = update_project_continuity(
                project_root=root,
                project_state_file=project_state_file,
                project_report_file=project_report_file,
                terminal_id="c-123",
                terminal_host="example-host",
                project_session_id="c-123-20260425T000000Z",
                cwd=root,
                observed_at="2026-04-25T00:00:00Z",
                important_files_limit=20,
                main_thread_id="main-thread",
            )
            second = update_project_continuity(
                project_root=root,
                project_state_file=project_state_file,
                project_report_file=project_report_file,
                terminal_id="c-123",
                terminal_host="example-host",
                project_session_id="c-123-20260425T000000Z",
                cwd=root,
                observed_at="2026-04-25T00:10:00Z",
                important_files_limit=20,
                main_thread_id="main-thread",
            )

            self.assertEqual(1, len(first["sessions"]))
            self.assertEqual(1, len(second["sessions"]))
            self.assertEqual(2, second["sessions"][0]["interaction_count"])
            self.assertEqual("2026-04-25T00:10:00Z", second["sessions"][0]["last_seen_at"])
            self.assertTrue(project_report_file.exists())
            report = project_report_file.read_text()
            self.assertIn("## Terminal Interaction Log", report)
            self.assertIn("c-123-20260425T000000Z", report)
            self.assertIn("## Important Files To Review", report)


if __name__ == "__main__":
    unittest.main()
