#!/usr/bin/env bash
set -euo pipefail

SESSION="brat"

if tmux has-session -t "$SESSION" 2>/dev/null; then
  echo "tmux session '$SESSION' already exists"
  exit 0
fi

tmux new-session -d -s "$SESSION" -n mayor

tmux send-keys -t "$SESSION":mayor.0 "brat status --watch" C-m

tmux split-window -h -t "$SESSION":mayor

tmux split-window -v -t "$SESSION":mayor.1

tmux send-keys -t "$SESSION":mayor.1 "brat convoy list --json" C-m

tmux send-keys -t "$SESSION":mayor.2 "brat task list --json --label status:blocked" C-m

tmux select-layout -t "$SESSION":mayor tiled

# Witness window

tmux new-window -t "$SESSION" -n witness

tmux send-keys -t "$SESSION":witness.0 "brat session list --json" C-m

tmux split-window -h -t "$SESSION":witness

tmux split-window -v -t "$SESSION":witness.1

tmux send-keys -t "$SESSION":witness.1 "brat session tail <session_id> --lines 200" C-m

tmux send-keys -t "$SESSION":witness.2 "brat witness run --once" C-m

tmux select-layout -t "$SESSION":witness tiled

# Refinery window

tmux new-window -t "$SESSION" -n refinery

tmux send-keys -t "$SESSION":refinery.0 "brat task list --label merge:queued --json" C-m

tmux split-window -h -t "$SESSION":refinery

tmux split-window -v -t "$SESSION":refinery.1

tmux send-keys -t "$SESSION":refinery.1 "brat task list --label merge:failed --json" C-m

tmux send-keys -t "$SESSION":refinery.2 "brat refinery run --once" C-m

tmux select-layout -t "$SESSION":refinery tiled

# Deacon window

tmux new-window -t "$SESSION" -n deacon

tmux send-keys -t "$SESSION":deacon.0 "brat lock status --json" C-m

tmux split-window -h -t "$SESSION":deacon

tmux split-window -v -t "$SESSION":deacon.1

tmux send-keys -t "$SESSION":deacon.1 "brat doctor --check --json" C-m

tmux send-keys -t "$SESSION":deacon.2 "brat sync --pull" C-m

tmux select-layout -t "$SESSION":deacon tiled

# Sessions window

tmux new-window -t "$SESSION" -n sessions

tmux send-keys -t "$SESSION":sessions.0 "brat session list --json" C-m

tmux split-window -h -t "$SESSION":sessions

tmux split-window -v -t "$SESSION":sessions.1

tmux send-keys -t "$SESSION":sessions.1 "brat task list --label status:running --json" C-m

tmux send-keys -t "$SESSION":sessions.2 "brat task list --label status:needs-review --json" C-m

tmux select-layout -t "$SESSION":sessions tiled

tmux select-window -t "$SESSION":mayor

tmux attach -t "$SESSION"
