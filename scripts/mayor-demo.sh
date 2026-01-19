#!/bin/bash
# Mayor Full Demo - Automated walkthrough of AI Mayor functionality
# This script demonstrates the complete Mayor workflow from setup to task creation
#
# Usage: ./scripts/mayor-demo.sh [OPTIONS]
#
# Options:
#   --with-ui    Also start the web UI dashboard (http://localhost:5173)
#   --ui-only    Only start the UI without running the demo
#   --help       Show this help message

# Exit on error for setup steps, but not for Mayor interactions
set -e

# Parse arguments
WITH_UI=false
UI_ONLY=false
for arg in "$@"; do
    case $arg in
        --with-ui)
            WITH_UI=true
            shift
            ;;
        --ui-only)
            UI_ONLY=true
            WITH_UI=true
            shift
            ;;
        --help|-h)
            echo "Usage: ./scripts/mayor-demo.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --with-ui    Also start the web UI dashboard (http://localhost:5173)"
            echo "  --ui-only    Only start the UI without running the demo"
            echo "  --help       Show this help message"
            exit 0
            ;;
    esac
done

# Trap to clean up on exit
cleanup() {
    if [ -n "$BRAT_BIN" ] && [ -d "/tmp/mayor-test-repo" ]; then
        cd /tmp/mayor-test-repo 2>/dev/null && "$BRAT_BIN" mayor stop 2>/dev/null || true
    fi
    # Stop daemon if we started it
    [ "$DAEMON_STARTED" = true ] && "$BRAT_BIN" daemon stop 2>/dev/null || true
    # Kill UI process if started
    [ ! -z "$UI_PID" ] && kill $UI_PID 2>/dev/null || true
}
trap cleanup EXIT

DAEMON_STARTED=false

# Configuration
TEST_DIR="/tmp/mayor-test-repo"
BRAT_BIN="${BRAT_BIN:-/home/dipankar/Code/brat/target/release/brat}"
BRAT_REPO="${BRAT_REPO:-/home/dipankar/Code/brat}"
UI_DIR="${BRAT_REPO}/brat-ui"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color
BOLD='\033[1m'
DIM='\033[2m'

# Helper functions
banner() {
    echo ""
    echo -e "${CYAN}${BOLD}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}${BOLD}║  $1$(printf '%*s' $((60 - ${#1})) '')║${NC}"
    echo -e "${CYAN}${BOLD}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

step() {
    echo -e "${GREEN}${BOLD}▶ STEP $1: $2${NC}"
    echo -e "${DIM}$(printf '─%.0s' {1..66})${NC}"
}

info() {
    echo -e "${BLUE}  ℹ $1${NC}"
}

success() {
    echo -e "${GREEN}  ✓ $1${NC}"
}

warning() {
    echo -e "${YELLOW}  ⚠ $1${NC}"
}

show_command() {
    echo -e "${YELLOW}  \$ $1${NC}"
}

run_brat() {
    show_command "brat $*"
    "$BRAT_BIN" "$@"
}

# Run brat command with retry on failure
run_brat_retry() {
    local max_attempts=3
    local attempt=1

    show_command "brat $*"

    while [ $attempt -le $max_attempts ]; do
        if "$BRAT_BIN" "$@" 2>&1; then
            return 0
        else
            if [ $attempt -lt $max_attempts ]; then
                echo -e "${YELLOW}  ⚠ Command failed, retrying (attempt $((attempt+1))/$max_attempts)...${NC}"
                sleep 2
            fi
        fi
        attempt=$((attempt + 1))
    done

    echo -e "${RED}  ✗ Command failed after $max_attempts attempts${NC}"
    return 1
}

divider() {
    echo ""
    echo -e "${DIM}────────────────────────────────────────────────────────────────────${NC}"
    echo ""
}

pause() {
    local seconds="${1:-3}"
    echo -e "${DIM}  (pausing ${seconds}s for readability...)${NC}"
    sleep "$seconds"
}

# Start UI servers (API + frontend dev server)
start_ui() {
    echo -e "${CYAN}${BOLD}Starting Web UI...${NC}"
    echo ""

    # Check if UI directory exists
    if [ ! -d "$UI_DIR" ]; then
        echo -e "${RED}  ✗ UI directory not found at $UI_DIR${NC}"
        echo -e "${YELLOW}  Run from the brat repository root${NC}"
        return 1
    fi

    # Check if node_modules exists
    if [ ! -d "$UI_DIR/node_modules" ]; then
        info "Installing UI dependencies..."
        cd "$UI_DIR"
        npm install --silent
    fi

    # Start daemon (bratd)
    info "Starting brat daemon on port 3000..."
    "$BRAT_BIN" daemon start > /tmp/brat-daemon.log 2>&1
    DAEMON_STARTED=true
    sleep 1

    # Check if daemon is running
    if ! "$BRAT_BIN" daemon status --quiet 2>/dev/null; then
        echo -e "${RED}  ✗ Failed to start daemon${NC}"
        cat /tmp/brat-daemon.log
        return 1
    fi
    success "Daemon running (use 'brat daemon status' to check)"

    # Start UI dev server
    info "Starting UI dev server on port 5173..."
    cd "$UI_DIR"
    npm run dev > /tmp/brat-ui.log 2>&1 &
    UI_PID=$!
    sleep 3

    if ! kill -0 $UI_PID 2>/dev/null; then
        echo -e "${RED}  ✗ Failed to start UI dev server${NC}"
        cat /tmp/brat-ui.log
        return 1
    fi
    success "UI dev server running (PID: $UI_PID)"

    echo ""
    echo -e "${GREEN}${BOLD}  Web UI available at: ${CYAN}http://localhost:5173${NC}"
    echo ""

    # Open browser
    if command -v xdg-open &> /dev/null; then
        xdg-open http://localhost:5173 2>/dev/null &
    elif command -v open &> /dev/null; then
        open http://localhost:5173 2>/dev/null &
    fi

    return 0
}

# UI-only mode
run_ui_only() {
    clear
    banner "BRAT WEB UI"

    echo -e "${BOLD}Starting the brat web dashboard...${NC}"
    echo ""
    echo "The UI provides:"
    echo "  • Dashboard with task status overview"
    echo "  • Convoy and task management"
    echo "  • Session monitoring with log viewer"
    echo "  • Mayor chat interface"
    echo ""
    divider

    if ! start_ui; then
        echo -e "${RED}Failed to start UI${NC}"
        exit 1
    fi

    echo -e "${BOLD}UI is running. Press Ctrl+C to stop.${NC}"
    echo ""
    echo "To register a repository in the UI:"
    echo "  1. Open http://localhost:5173"
    echo "  2. Click 'Add Repo' in the header"
    echo "  3. Enter the path to your brat-initialized repo"
    echo ""

    # Wait forever
    while true; do
        sleep 10
    done
}

# ============================================================================
# MAIN DEMO
# ============================================================================

# Handle UI-only mode
if [ "$UI_ONLY" = true ]; then
    run_ui_only
    exit 0
fi

clear
banner "AI MAYOR DEMONSTRATION"

echo -e "${BOLD}The Mayor is an AI orchestrator that:${NC}"
echo "  • Analyzes codebases to identify issues"
echo "  • Breaks down work into convoys (groups) and tasks"
echo "  • Coordinates autonomous coding agents"
echo "  • Monitors progress and reports status"
echo ""
echo -e "${DIM}This demo will walk through the complete Mayor workflow.${NC}"
divider

# ============================================================================
# STEP 1: Setup Test Repository
# ============================================================================

step "1" "Setting up test repository"

info "Creating test repo at $TEST_DIR with sample Python code"
info "This includes intentional bugs and TODOs for the Mayor to find"
echo ""

# Clean previous test
if [ -d "$TEST_DIR" ]; then
    rm -rf "$TEST_DIR"
fi

mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Initialize git
git init --quiet
git config user.email "test@brat.dev"
git config user.name "Brat Test"

# Create sample codebase
mkdir -p src/tests

cat > src/main.py << 'EOF'
"""Main entry point for the sample application."""
# TODO: Add argument parsing with argparse
# TODO: Add logging with proper log levels

from utils import process_data, format_output

def main():
    """Process some sample data."""
    data = {"items": [1, 2, 3, 4, 5]}
    result = process_data(data)
    output = format_output(result)
    print(f"Result: {output}")

if __name__ == "__main__":
    main()
EOF

cat > src/utils.py << 'EOF'
"""Utility functions for data processing."""

def process_data(data):
    """Process the input data and return the average."""
    # BUG: No input validation - crashes on empty list (ZeroDivisionError)
    # BUG: No type checking - crashes on non-dict input
    items = data["items"]
    return sum(items) / len(items)

def format_output(value):
    """Format a numeric value for display."""
    # TODO: Add proper number formatting
    return str(value)

def validate_data(data):
    """Validate input data structure."""
    # TODO: Implement validation logic
    pass
EOF

cat > src/tests/test_utils.py << 'EOF'
"""Tests for utility functions."""
import unittest
import sys
sys.path.insert(0, '..')

from utils import process_data

class TestUtils(unittest.TestCase):
    def test_basic_average(self):
        result = process_data({"items": [1, 2, 3]})
        self.assertEqual(result, 2.0)

    # TODO: Add edge case tests
    # TODO: Add error handling tests

if __name__ == "__main__":
    unittest.main()
EOF

# Initial commit
git add .
git commit -m "Initial commit: sample Python project" --quiet

# Initialize grit
grit init >/dev/null 2>&1

# Initialize brat
"$BRAT_BIN" init --no-daemon --no-tmux >/dev/null 2>&1

# Copy workflows
mkdir -p "$TEST_DIR/.brat/workflows"
if [ -d "$BRAT_REPO/.brat/workflows" ]; then
    cp -r "$BRAT_REPO/.brat/workflows"/* "$TEST_DIR/.brat/workflows/" 2>/dev/null || true
fi

success "Test repository created at $TEST_DIR"
success "Sample codebase includes bugs and TODOs"
echo ""
echo -e "${MAGENTA}  Sample code structure:${NC}"
echo "    src/main.py      - Entry point with TODOs"
echo "    src/utils.py     - Has ZeroDivisionError bug"
echo "    src/tests/       - Incomplete test suite"

divider
pause 2

# ============================================================================
# STEP 1.5: Start UI (if --with-ui)
# ============================================================================

if [ "$WITH_UI" = true ]; then
    step "1.5" "Starting Web UI Dashboard"

    info "The UI provides a visual interface for monitoring brat"
    echo ""

    # Register the test repo with the API
    if start_ui; then
        sleep 2
        # Register the test repo via API
        info "Registering test repository with API..."
        curl -s -X POST http://localhost:3000/api/v1/repos \
            -H "Content-Type: application/json" \
            -d "{\"path\": \"$TEST_DIR\"}" > /dev/null 2>&1 || true
        success "Test repo registered with API"
        echo ""
        echo -e "${CYAN}${BOLD}  You can now view the demo in real-time at:${NC}"
        echo -e "${CYAN}${BOLD}  http://localhost:5173${NC}"
        echo ""
        info "The UI will update automatically as the demo progresses"
    else
        warning "Could not start UI, continuing with CLI demo only"
    fi

    divider
    pause 3
fi

# ============================================================================
# STEP 2: Start Mayor
# ============================================================================

# From here on, don't exit on errors - Mayor commands may fail and we want to continue
set +e

step "2" "Starting the AI Mayor"

info "The Mayor is a Claude Code session with special context"
info "It has access to brat CLI commands for creating convoys/tasks"
echo ""

# Stop any existing mayor
"$BRAT_BIN" mayor stop 2>/dev/null || true

run_brat mayor start

success "Mayor session initialized"
divider
pause 2

# ============================================================================
# STEP 3: View Mayor's Initial Response
# ============================================================================

step "3" "Viewing Mayor's initial response"

info "The Mayor reads .claude/mayor_context.md to understand its role"
echo ""

run_brat mayor tail -n 30

divider
pause 3

# ============================================================================
# STEP 4: Ask Mayor to Analyze Codebase
# ============================================================================

step "4" "Asking Mayor to analyze the codebase"

info "Sending: 'Analyze src/ and list all bugs and issues you find'"
echo ""

if ! run_brat_retry mayor ask "Analyze the src/ directory. List all bugs and issues you find, organized by severity (critical, high, medium, low)."; then
    warning "Mayor command failed. Restarting Mayor and retrying..."
    "$BRAT_BIN" mayor stop 2>/dev/null || true
    sleep 1
    "$BRAT_BIN" mayor start 2>/dev/null
    sleep 2
    run_brat mayor ask "Analyze the src/ directory. List all bugs and issues you find, organized by severity (critical, high, medium, low)."
fi

divider
pause 3

# ============================================================================
# STEP 5: Ask Mayor to Create Convoy and Tasks
# ============================================================================

step "5" "Asking Mayor to create a convoy with tasks"

info "The Mayor will use 'brat convoy create' and 'brat task create'"
info "to organize work for autonomous coding agents"
echo ""

if ! run_brat_retry mayor ask "Create a convoy called 'Bug Fixes' with tasks to fix the critical and high severity bugs. Use the brat CLI commands."; then
    warning "Mayor command failed. Restarting Mayor and retrying..."
    "$BRAT_BIN" mayor stop 2>/dev/null || true
    sleep 1
    "$BRAT_BIN" mayor start 2>/dev/null
    sleep 2
    run_brat mayor ask "Create a convoy called 'Bug Fixes' with tasks to fix the bugs you analyzed earlier. Use brat convoy create and brat task create."
fi

divider
pause 3

# ============================================================================
# STEP 6: Check Brat Status
# ============================================================================

step "6" "Checking brat status"

info "Viewing convoys and tasks created by the Mayor"
echo ""

run_brat status

divider
pause 2

# ============================================================================
# STEP 7: View Grit Issues (Raw Data)
# ============================================================================

step "7" "Viewing raw grit issues"

info "Grit stores convoys and tasks as labeled issues"
echo ""

show_command "grit issue list"
grit issue list 2>/dev/null | head -20

divider
pause 2

# ============================================================================
# STEP 8: View Full Mayor Conversation
# ============================================================================

step "8" "Viewing full Mayor conversation history"

info "The Mayor maintains conversation context across calls"
echo ""

run_brat mayor tail -n 80

divider
pause 2

# ============================================================================
# STEP 9: Ask Mayor for Status Summary
# ============================================================================

step "9" "Asking Mayor for a status summary"

info "The Mayor can check and report on current work status"
echo ""

run_brat_retry mayor ask "Give me a brief summary of what we've accomplished and what tasks are ready for agents to work on." || true

divider
pause 2

# ============================================================================
# STEP 10: Stop Mayor
# ============================================================================

step "10" "Stopping the Mayor"

info "Cleaning up the Mayor session"
echo ""

run_brat mayor stop

success "Mayor session stopped"
divider

# ============================================================================
# SUMMARY
# ============================================================================

banner "DEMO COMPLETE"

echo -e "${BOLD}What we demonstrated:${NC}"
echo ""
echo "  1. Created a test repository with sample Python code"
echo "  2. Initialized brat harness (git + grit + brat)"
if [ "$WITH_UI" = true ]; then
echo "  3. Started the Web UI dashboard"
echo "  4. Started the AI Mayor (Claude Code orchestrator)"
echo "  5. Mayor analyzed the codebase and found bugs"
echo "  6. Mayor created a convoy with tasks for bug fixes"
echo "  7. Viewed the work breakdown in brat status"
echo "  8. Stopped the Mayor session"
else
echo "  3. Started the AI Mayor (Claude Code orchestrator)"
echo "  4. Mayor analyzed the codebase and found bugs"
echo "  5. Mayor created a convoy with tasks for bug fixes"
echo "  6. Viewed the work breakdown in brat status"
echo "  7. Stopped the Mayor session"
fi
echo ""
echo -e "${BOLD}Key concepts:${NC}"
echo ""
echo "  • ${CYAN}Mayor${NC}     - AI orchestrator that breaks down work"
echo "  • ${CYAN}Convoy${NC}    - Group of related tasks"
echo "  • ${CYAN}Task${NC}      - Individual work item for an agent"
echo "  • ${CYAN}Witness${NC}   - Workflow that spawns agents for tasks"
echo ""
echo -e "${BOLD}Next steps to try:${NC}"
echo ""
echo "  # Re-run this demo with the web UI"
echo "  ./scripts/mayor-demo.sh --with-ui"
echo ""
echo "  # Run just the web UI (standalone)"
echo "  ./scripts/mayor-demo.sh --ui-only"
echo ""
echo "  # Manual Mayor interaction"
echo "  cd $TEST_DIR"
echo "  brat mayor start"
echo "  brat mayor ask 'your question here'"
echo ""
echo "  # Spawn agents for queued tasks (requires witness)"
echo "  brat witness run --once"
echo ""
echo "  # Watch activity dashboard"
echo "  ./scripts/mayor-watch.sh"
echo ""

if [ "$WITH_UI" = true ] && [ ! -z "$UI_PID" ]; then
echo -e "${CYAN}${BOLD}The Web UI is still running at: http://localhost:5173${NC}"
echo -e "${DIM}Press Ctrl+C to stop the UI servers${NC}"
echo ""
# Keep running so user can explore UI
echo -e "${BOLD}Explore the UI to see the results of the demo!${NC}"
echo ""
while true; do
    sleep 10
done
else
echo -e "${GREEN}${BOLD}Thanks for trying the AI Mayor!${NC}"
echo ""
fi
