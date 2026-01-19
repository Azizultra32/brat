#!/bin/bash
# Mayor Activity Dashboard
# A tmux-based visualization showing Mayor activity in real-time

TEST_DIR="${1:-/tmp/mayor-test-repo}"
SESSION="mayor-watch"

# Check prerequisites
if ! command -v tmux &> /dev/null; then
    echo "Error: tmux is required but not installed"
    exit 1
fi

if [ ! -d "$TEST_DIR" ]; then
    echo "Error: Test directory not found: $TEST_DIR"
    echo "Run ./scripts/mayor-test-setup.sh first"
    exit 1
fi

cd "$TEST_DIR" || exit 1

echo "Starting Mayor Activity Dashboard..."
echo "Directory: $TEST_DIR"
echo ""

# Kill existing session
tmux kill-session -t $SESSION 2>/dev/null

# Create new session with dashboard layout
tmux new-session -d -s $SESSION -n "Mayor Dashboard" -x 200 -y 50

# Layout:
# +---------------------------+------------------+
# |                           |                  |
# |   Mayor Conversation      |   Brat Status    |
# |   (tail -f style)         |   (JSON)         |
# |                           |                  |
# +---------------------------+------------------+
# |                           |                  |
# |   Convoy/Task Status      |   Sessions       |
# |                           |                  |
# +---------------------------+------------------+

# Pane 0: Mayor conversation (top-left, main focus)
tmux send-keys -t $SESSION "cd $TEST_DIR && clear && echo '=== MAYOR CONVERSATION ===' && echo '' && while true; do brat mayor tail -n 30 2>/dev/null || echo '(Mayor not running)'; sleep 3; clear; echo '=== MAYOR CONVERSATION ==='; echo ''; done" Enter

# Pane 1: Status JSON (top-right)
tmux split-window -h -t $SESSION -p 35
tmux send-keys -t $SESSION "cd $TEST_DIR && watch -n 2 -c 'echo \"=== BRAT STATUS (JSON) ===\"; echo \"\"; brat status --json 2>/dev/null | jq -C . 2>/dev/null || brat status 2>/dev/null || echo \"(brat not initialized)\"'" Enter

# Pane 2: Human-readable status (bottom-left)
tmux select-pane -t 0
tmux split-window -v -t $SESSION -p 40
tmux send-keys -t $SESSION "cd $TEST_DIR && watch -n 2 'echo \"=== CONVOYS & TASKS ===\"; echo \"\"; brat status 2>/dev/null || echo \"(no convoys)\"'" Enter

# Pane 3: Sessions (bottom-right)
tmux select-pane -t 2
tmux split-window -v -t $SESSION -p 50
tmux send-keys -t $SESSION "cd $TEST_DIR && watch -n 2 'echo \"=== ACTIVE SESSIONS ===\"; echo \"\"; brat session list 2>/dev/null || echo \"(no sessions)\"'" Enter

# Select the main pane (Mayor conversation)
tmux select-pane -t 0

echo "Dashboard started in tmux session: $SESSION"
echo ""
echo "Controls:"
echo "  Ctrl+B then arrow keys - switch panes"
echo "  Ctrl+B then d          - detach (dashboard keeps running)"
echo "  Ctrl+B then [          - scroll mode (q to exit)"
echo "  Ctrl+C in pane         - stop that watch"
echo ""
echo "Attaching to dashboard..."
sleep 1

# Attach to session
tmux attach -t $SESSION
