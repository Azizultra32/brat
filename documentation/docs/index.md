# Brat

**Orchestrate autonomous AI coding agents with crash-safe, deterministic state management.**

Brat is a multi-agent harness that coordinates AI coding tools (Claude Code, Aider, Codex, and more) working in parallel on your codebase. Built on [Grite](https://github.com/neul-labs/grite), an append-only event log, Brat ensures that even if agents crash, your coordination state is always recoverable and auditable.

---

## Key Features

<div class="grid cards" markdown>

-   :material-robot-outline:{ .lg .middle } **Multi-Agent Orchestration**

    ---

    Coordinate Claude Code, Aider, Codex, and other AI coding tools working together on your codebase.

-   :material-shield-check:{ .lg .middle } **Crash-Safe State**

    ---

    All coordination state lives in an append-only event log. Recover deterministically from any crash.

-   :material-view-dashboard:{ .lg .middle } **Web Dashboard**

    ---

    Real-time monitoring and control via a modern web UI at `localhost:5173`.

-   :material-source-merge:{ .lg .middle } **Merge Management**

    ---

    Refinery manages the merge queue with configurable policies (rebase, squash, merge).

</div>

---

## Quick Example

```bash
# Initialize Brat in your project
cd your-project
grite init      # Initialize Grite substrate
brat init      # Initialize Brat harness

# Start the AI orchestrator
brat mayor start
brat mayor ask "Analyze src/ and create tasks for any bugs you find"

# View status
brat status

# Spawn agents to work on tasks
brat witness run --once
```

---

## How It Works

```
┌──────────┐         ┌──────────┐         ┌──────────┐
│  Mayor   │─creates─▶│  Convoy  │─contains─▶│  Tasks   │
│  (AI)    │         │  (group) │         │  (work)  │
└──────────┘         └──────────┘         └────┬─────┘
                                               │
                     ┌─────────────────────────┘
                     ▼
              ┌─────────────┐      ┌─────────────┐
              │   Witness   │─────▶│  Refinery   │
              │(spawn agents)│      │(merge work) │
              └─────────────┘      └─────────────┘
```

| Role | What It Does |
|------|--------------|
| **Mayor** | AI orchestrator that analyzes codebases, breaks down work, and creates convoys/tasks |
| **Convoy** | A group of related tasks (think: sprint, epic, or feature branch) |
| **Task** | Individual work item assigned to an AI coding agent |
| **Witness** | Spawns and monitors coding agent sessions |
| **Refinery** | Manages the merge queue, runs CI checks, handles integration |

---

## Get Started

<div class="grid cards" markdown>

-   :material-download:{ .lg .middle } **Installation**

    ---

    Install Brat and Grite on your system.

    [:octicons-arrow-right-24: Install now](getting-started/installation.md)

-   :material-rocket-launch:{ .lg .middle } **Quickstart**

    ---

    Get up and running in 5 minutes.

    [:octicons-arrow-right-24: Quickstart guide](getting-started/quickstart.md)

-   :material-school:{ .lg .middle } **Your First Convoy**

    ---

    Step-by-step tutorial for creating your first convoy.

    [:octicons-arrow-right-24: Tutorial](getting-started/first-convoy.md)

-   :material-book-open-variant:{ .lg .middle } **Concepts**

    ---

    Understand how Brat works under the hood.

    [:octicons-arrow-right-24: Learn concepts](concepts/index.md)

</div>

---

## Supported AI Engines

Brat works with your preferred AI coding tool:

| Engine | Command | Highlights |
|--------|---------|------------|
| **Claude Code** | `claude` | Native Anthropic integration, session resume |
| **Aider** | `aider` | Multi-model support (GPT-4, Claude, Gemini, local LLMs) |
| **OpenCode** | `opencode` | 75+ LLM providers, open-source Claude Code alternative |
| **Codex** | `codex` | Structured JSON output for parsing |
| **Continue** | `cn` | IDE integration, CI/CD pipelines |
| **Gemini** | `gemini` | Google's free tier |
| **GitHub Copilot** | `gh copilot` | Shell/git command suggestions |

Configure your engine in `.brat/config.toml` and Brat handles the rest.
