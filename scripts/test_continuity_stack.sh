#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

main_thread="${MAIN_THREAD_ID:-019db40e-5f2b-7032-b825-dd1541ff6ea2}"
worker_threads=(
  "${WORKER_THREAD_1:-019db446-177e-74b1-9da2-998439ab4618}"
  "${WORKER_THREAD_2:-019db478-a2c4-7f42-8479-11be6dd10c54}"
  "${WORKER_THREAD_3:-019db485-7b6f-7890-a8d8-4bc71f73de4e}"
  "${WORKER_THREAD_4:-019db488-a025-74b0-a07d-912e61347041}"
  "${WORKER_THREAD_5:-019db490-10da-7e70-b8ea-8dfe7d11464c}"
  "${WORKER_THREAD_6:-019db648-f3f2-7b73-bfe1-b81ec39fa52a}"
)

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/continuity-stack.XXXXXX")"
cleanup() {
  tmux kill-session -t continuity-supervisor-smoke-test >/dev/null 2>&1 || true
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

assert_file() {
  local path="$1"
  [ -f "${path}" ] || {
    echo "missing required file: ${path}" >&2
    exit 1
  }
}

assert_contains() {
  local needle="$1"
  local path="$2"
  grep -Fq "${needle}" "${path}" || {
    echo "expected '${needle}' in ${path}" >&2
    exit 1
  }
}

printf '==> unit tests\n'
python3 -m unittest discover -s tests -p 'test_continuity_supervisor.py'

printf '==> python compile\n'
python3 -m py_compile scripts/context_pool_watch.py scripts/session_handoff.py scripts/continuity_supervisor.py

printf '==> one-shot pool snapshot\n'
pool_args=(--once --thread "${main_thread}")
for thread in "${worker_threads[@]}"; do
  pool_args+=(--thread "${thread}")
done
python3 scripts/context_pool_watch.py "${pool_args[@]}" > "${tmpdir}/pool-watch.out"
assert_contains "threshold=70.0%" "${tmpdir}/pool-watch.out"

printf '==> one-shot handoff\n'
handoff_args=(--output "${tmpdir}/session-handoff.md" --thread "${main_thread}")
for thread in "${worker_threads[@]}"; do
  handoff_args+=(--thread "${thread}")
done
python3 scripts/session_handoff.py "${handoff_args[@]}" > "${tmpdir}/session-handoff.path"
assert_file "${tmpdir}/session-handoff.md"
assert_contains "# Session Handoff" "${tmpdir}/session-handoff.md"
assert_contains "${main_thread}" "${tmpdir}/session-handoff.md"

printf '==> one-shot supervisor\n'
supervisor_args=(
  --once
  --main-thread "${main_thread}"
  --project-root "${repo_root}"
  --cwd "${repo_root}"
  --terminal-id "continuity-test-terminal"
  --terminal-host "continuity-test-host"
  --project-session-id "continuity-test-session"
  --state-file "${tmpdir}/context-pool.json"
  --events-file "${tmpdir}/context-pool-events.jsonl"
  --handoff-file "${tmpdir}/session-handoff-supervisor.md"
  --supervisor-file "${tmpdir}/continuity-supervisor.json"
  --companion-file "${tmpdir}/continuity-companion.md"
  --supervisor-events-file "${tmpdir}/continuity-supervisor-events.jsonl"
  --project-state-file "${tmpdir}/project-continuity.json"
  --project-report-file "${tmpdir}/project-continuity.md"
)
for thread in "${worker_threads[@]}"; do
  supervisor_args+=(--worker-thread "${thread}")
done
python3 scripts/continuity_supervisor.py "${supervisor_args[@]}" > "${tmpdir}/supervisor.out"
assert_contains "doc_companion=active" "${tmpdir}/supervisor.out"
assert_file "${tmpdir}/continuity-supervisor.json"
assert_file "${tmpdir}/continuity-companion.md"
assert_file "${tmpdir}/project-continuity.json"
assert_file "${tmpdir}/project-continuity.md"
assert_contains "\"required_caretakers\"" "${tmpdir}/continuity-supervisor.json"
assert_contains "## Terminal Interaction Log" "${tmpdir}/project-continuity.md"
assert_contains "continuity-test-session" "${tmpdir}/project-continuity.md"
assert_contains "## Important Files To Review" "${tmpdir}/project-continuity.md"
assert_contains "AGENTS.md" "${tmpdir}/project-continuity.md"

printf '==> launcher smoke\n'
smoke_log="${tmpdir}/continuity-supervisor.log"
SESSION_NAME=continuity-supervisor-smoke-test \
CONTINUITY_SUPERVISOR_LOG_FILE="${smoke_log}" \
TERMINAL_ID="continuity-test-terminal" \
TERMINAL_HOST="continuity-test-host" \
PROJECT_SESSION_ID="continuity-smoke-session" \
scripts/start_continuity_supervisor.sh \
  --main-thread "${main_thread}" \
  --worker-thread "${worker_threads[0]}" \
  --worker-thread "${worker_threads[1]}" \
  --worker-thread "${worker_threads[2]}" \
  --worker-thread "${worker_threads[3]}" \
  --worker-thread "${worker_threads[4]}" \
  --worker-thread "${worker_threads[5]}" \
  > "${tmpdir}/launcher.out"

for _ in {1..10}; do
  if [ -f "${smoke_log}" ] && grep -Fq "doc_companion=active" "${smoke_log}"; then
    break
  fi
  sleep 1
done

assert_file "${smoke_log}"
assert_contains "doc_companion=active" "${smoke_log}"
assert_file "${repo_root}/.brat/continuity/project-continuity.md"
assert_contains "## Terminal Interaction Log" "${repo_root}/.brat/continuity/project-continuity.md"

tmux kill-session -t continuity-supervisor-smoke-test >/dev/null 2>&1 || true

printf '==> ok\n'
echo "continuity stack smoke passed"
