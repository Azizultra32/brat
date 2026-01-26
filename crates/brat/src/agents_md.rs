//! AGENTS.md template content for brat section.
//!
//! This module provides the template content that brat adds to AGENTS.md
//! during initialization. This follows grite's pattern of having each tool
//! add its own section to AGENTS.md.

/// Template content for the Brat section in AGENTS.md
pub const BRAT_AGENTS_SECTION: &str = r#"## Brat

This repository uses **Brat** for AI agent orchestration. Use brat commands for work management.

### Checking Status

```bash
# Overall status (convoys, tasks, sessions)
brat status --json

# List active sessions
brat session list --json
```

### Working on Tasks

When assigned a task, you'll have task context in `.claude/current_task.md`.

```bash
# Update task status as you progress
brat task update <task_id> --status running --json
brat task update <task_id> --status needs-review --json

# Add progress comments (recorded in grite issue)
grite issue comment <issue_id> --body "Checkpoint: implemented X, tests passing" --json
```

### Storing Context

Store architectural observations for future agents:

```bash
# Project-level context (conventions, patterns)
grite context set test_command "cargo test"
grite context set build_command "cargo build"
grite context set api_pattern "REST /api/v1/"

# Discoveries as memory issues
grite issue create --label memory --title "[Memory] Auth flow" --body "Uses JWT..." --json

# Index codebase symbols after significant changes
grite context index --json
```

### Reading Context

Query existing knowledge before starting:

```bash
# Project conventions
grite context project --json

# Previous discoveries
grite issue list --label memory --json

# Code symbols
grite context query <function_name> --json
```
"#;
