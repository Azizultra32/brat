#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 --main-thread <thread-id> [--supervisor-thread <id> ...] [--worker-thread <id> ...]" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
session_name="${SESSION_NAME:-continuity-supervisor}"
log_file="${HOME}/.codex/continuity-supervisor.log"

mkdir -p "${HOME}/.codex"

if tmux has-session -t "${session_name}" 2>/dev/null; then
  tmux kill-session -t "${session_name}"
fi

cmd=(python3 "${repo_root}/scripts/continuity_supervisor.py" "$@")
quoted_cmd="$(printf '%q ' "${cmd[@]}")"
quoted_repo_root="$(printf '%q' "${repo_root}")"
quoted_log_file="$(printf '%q' "${log_file}")"

tmux new-session -d -s "${session_name}" "cd ${quoted_repo_root} && ${quoted_cmd} >> ${quoted_log_file} 2>&1"
echo "${session_name}"
