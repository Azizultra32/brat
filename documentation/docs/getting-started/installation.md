# Installation

This guide covers installing Brat and its dependencies.

## Prerequisites

- **Rust toolchain** (1.75 or later)
- **Git** (2.30 or later)

## Install Grit

Brat is built on [Grit](https://github.com/neul-labs/grit), an append-only event log. Install Grit first:

```bash
cargo install --git https://github.com/neul-labs/grit grit
```

Verify the installation:

```bash
grit --version
```

## Install Brat

### Option 1: One-Line Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/brat/main/install.sh | bash
```

### Option 2: From Source

Clone and build:

```bash
git clone https://github.com/neul-labs/brat
cd brat
cargo install --path crates/brat
```

### Option 3: Cargo Install

```bash
cargo install brat
```

## Verify Installation

Check that both tools are installed:

```bash
grit --version
brat --version
```

## Install an AI Engine

Brat orchestrates AI coding tools. Install at least one:

=== "Claude Code"

    ```bash
    # Install via npm
    npm install -g @anthropic-ai/claude-code

    # Or via homebrew
    brew install claude-code
    ```

=== "Aider"

    ```bash
    pip install aider-chat
    ```

=== "Codex"

    ```bash
    npm install -g @openai/codex
    ```

=== "OpenCode"

    ```bash
    cargo install opencode
    ```

## Web UI (Optional)

To use the web dashboard, install the UI dependencies:

```bash
cd brat/brat-ui
npm install
```

Start the UI with:

```bash
npm run dev
```

The dashboard will be available at `http://localhost:5173`.

## Next Steps

Now that Brat is installed, continue to the [Quickstart](quickstart.md) guide.
