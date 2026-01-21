# Web Dashboard

Brat includes a web dashboard for monitoring and controlling agents.

## Starting the Dashboard

### With the Demo Script

```bash
./scripts/ui-demo.sh
```

Opens the dashboard at `http://localhost:5173`.

### Manually

1. Start the daemon:
   ```bash
   brat daemon start
   ```

2. Start the UI:
   ```bash
   cd brat-ui
   npm install
   npm run dev
   ```

3. Open `http://localhost:5173`

## Dashboard Overview

The dashboard has several tabs:

| Tab | Purpose |
|-----|---------|
| **Dashboard** | Overview with task status cards |
| **Convoys** | Create and manage convoys |
| **Tasks** | Filter, assign, and track tasks |
| **Sessions** | Monitor active AI agents |
| **Mayor Chat** | Interactive Mayor interface |

## Dashboard Tab

The main dashboard shows:

- **Task Status Cards** - Counts by status (queued, running, blocked, merged)
- **Recent Activity** - Latest task and session updates
- **Quick Actions** - Common operations

## Convoys Tab

### Creating a Convoy

1. Click **Create Convoy**
2. Enter title and goal
3. Click **Create**

### Viewing Convoy Details

Click a convoy to see:

- All tasks in the convoy
- Task status breakdown
- Convoy metadata

## Tasks Tab

### Filtering Tasks

Use filters to find tasks:

- **Status**: queued, running, blocked, etc.
- **Priority**: P0, P1, P2
- **Convoy**: Filter by parent convoy

### Task Details

Click a task to see:

- Title and paths
- Current status
- Assigned agent
- Comments and history

### Actions

- **Assign** - Assign to an agent
- **Comment** - Add context
- **Close** - Mark as done or dropped

## Sessions Tab

### Active Sessions

View all running agent sessions:

- Session ID
- Task being worked on
- Last heartbeat
- Health status

### Session Logs

Click a session to view live logs from the agent.

### Stopping Sessions

Click **Stop** to terminate a session.

## Mayor Chat Tab

Interactive interface to communicate with the Mayor.

### Sending Messages

Type in the input field and press Enter or click Send.

### Example Interactions

```
You: Analyze src/ and identify bugs
Mayor: Found 3 issues...

You: Create a convoy for the critical bugs
Mayor: Created convoy-abc123 with 2 tasks...
```

## Real-Time Updates

The dashboard updates in real-time via WebSocket connection:

- Task status changes appear immediately
- Session heartbeats update live
- New convoys/tasks appear automatically

## Troubleshooting

### Dashboard Not Loading

Check if the daemon is running:

```bash
brat daemon status
```

Start if needed:

```bash
brat daemon start
```

### No Data Showing

Ensure you're connected to the right repository:

1. Check the daemon port
2. Verify the daemon is serving your repo

### Stale Data

Click the refresh button or reload the page.

## Configuration

### Custom Port

Start the daemon on a different port:

```bash
brat daemon start --port 8080
```

Update the UI configuration to match.

### API Endpoint

The UI connects to the daemon API at `http://localhost:3000` by default.

Configure in `brat-ui/.env`:

```
VITE_API_URL=http://localhost:8080
```
