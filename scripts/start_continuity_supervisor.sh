#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 --main-thread <thread-id> [--supervisor-thread <id> ...] [--worker-thread <id> ...]" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
session_name="${SESSION_NAME:-continuity-supervisor}"
log_file="${HOME}/.codex/continuity-supervisor.log"
terminal_id="${TERMINAL_ID:-$(cat "${HOME}/.codex/.tid_current" 2>/dev/null || true)}"
terminal_host="${TERMINAL_HOST:-$(hostname)}"
project_session_id="${PROJECT_SESSION_ID:-${terminal_id:-unknown-terminal}-$(date -u +%Y%m%dT%H%M%SZ)}"

mkdir -p "${HOME}/.codex"

if tmux has-session -t "${session_name}" 2>/dev/null; then
  tmux kill-session -t "${session_name}"
fi

cmd=(
  python3 "${repo_root}/scripts/continuity_supervisor.py"
  --project-root "${repo_root}"
  --cwd "${PWD}"
  --terminal-id "${terminal_id}"
  --terminal-host "${terminal_host}"
  --project-session-id "${project_session_id}"
  "$@"
)
quoted_cmd="$(printf '%q ' "${cmd[@]}")"
quoted_repo_root="$(printf '%q' "${repo_root}")"
quoted_log_file="$(printf '%q' "${log_file}")"

tmux new-session -d -s "${session_name}" "cd ${quoted_repo_root} && ${quoted_cmd} >> ${quoted_log_file} 2>&1"
echo "${session_name}"
