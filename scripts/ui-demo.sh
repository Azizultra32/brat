#!/bin/bash
# UI Demo - Start API server and UI dev server
#
# Usage: ./scripts/ui-demo.sh [--build-only]
#
# This script starts both the brat API server and the brat-ui dev server
# for local development and testing.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BRAT_BIN="${BRAT_BIN:-$PROJECT_ROOT/target/release/brat}"
UI_DIR="$PROJECT_ROOT/brat-ui"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}Shutting down servers...${NC}"
    [ ! -z "$API_PID" ] && kill $API_PID 2>/dev/null
    [ ! -z "$UI_PID" ] && kill $UI_PID 2>/dev/null
    echo -e "${GREEN}Done.${NC}"
}
trap cleanup EXIT

# Check if brat binary exists
if [ ! -f "$BRAT_BIN" ]; then
    echo -e "${YELLOW}Building brat binary...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release -p brat
fi

# Check if UI dependencies are installed
if [ ! -d "$UI_DIR/node_modules" ]; then
    echo -e "${YELLOW}Installing UI dependencies...${NC}"
    cd "$UI_DIR"
    npm install
fi

# Build-only mode
if [ "$1" == "--build-only" ]; then
    echo -e "${YELLOW}Building UI for production...${NC}"
    cd "$UI_DIR"
    npm run build
    echo -e "${GREEN}Build complete! Files in $UI_DIR/dist/${NC}"
    exit 0
fi

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}  BRAT UI DEMO${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# Start API server
echo -e "${YELLOW}Starting brat API server on port 3000...${NC}"
"$BRAT_BIN" api &
API_PID=$!
sleep 2

# Check if API started successfully
if ! kill -0 $API_PID 2>/dev/null; then
    echo -e "${RED}Failed to start API server${NC}"
    exit 1
fi

# Start UI dev server
echo -e "${YELLOW}Starting brat-ui dev server on port 5173...${NC}"
cd "$UI_DIR"
npm run dev &
UI_PID=$!
sleep 2

# Check if UI started successfully
if ! kill -0 $UI_PID 2>/dev/null; then
    echo -e "${RED}Failed to start UI dev server${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}Servers are running:${NC}"
echo ""
echo -e "  API Server: ${GREEN}http://localhost:3000${NC}"
echo -e "  UI Server:  ${GREEN}http://localhost:5173${NC}"
echo ""
echo -e "  Press ${YELLOW}Ctrl+C${NC} to stop"
echo ""

# Open browser (optional)
if command -v xdg-open &> /dev/null; then
    sleep 1 && xdg-open http://localhost:5173 &
elif command -v open &> /dev/null; then
    sleep 1 && open http://localhost:5173 &
fi

# Wait for signals
wait
