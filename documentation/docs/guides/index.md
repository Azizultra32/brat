# User Guides

In-depth guides for working with Brat's features.

## Guides by Topic

<div class="grid cards" markdown>

-   :material-robot:{ .lg .middle } **Using the Mayor**

    ---

    Start the AI orchestrator, ask questions, and create work.

    [:octicons-arrow-right-24: Guide](using-the-mayor.md)

-   :material-truck-delivery:{ .lg .middle } **Managing Convoys**

    ---

    Create, monitor, pause, and complete convoys.

    [:octicons-arrow-right-24: Guide](managing-convoys.md)

-   :material-checkbox-marked:{ .lg .middle } **Managing Tasks**

    ---

    Add tasks, assign agents, track progress, and close.

    [:octicons-arrow-right-24: Guide](managing-tasks.md)

-   :material-view-dashboard:{ .lg .middle } **Web Dashboard**

    ---

    Monitor and control agents through the web UI.

    [:octicons-arrow-right-24: Guide](web-dashboard.md)

-   :material-file-tree:{ .lg .middle } **Workflow Templates**

    ---

    Create reusable workflows for common patterns.

    [:octicons-arrow-right-24: Guide](workflows.md)

-   :material-wrench:{ .lg .middle } **Troubleshooting**

    ---

    Common issues and how to fix them.

    [:octicons-arrow-right-24: Guide](troubleshooting.md)

</div>

## Common Workflows

### Bug Fixing

1. Start the Mayor: `brat mayor start`
2. Ask to analyze: `brat mayor ask "Analyze src/ for bugs"`
3. Review tasks: `brat status`
4. Spawn agents: `brat witness run --once`
5. Merge fixes: `brat refinery run --once`

### Feature Development

1. Create convoy: `brat convoy create --title "New feature" --goal "Add X"`
2. Add tasks: `brat task add --convoy <id> --title "Implement Y"`
3. Run agents: `brat witness run --once`
4. Review and merge: `brat refinery run --once`

### Code Review

1. Start Mayor: `brat mayor start`
2. Ask for review: `brat mayor ask "Review the changes in src/"`
3. View feedback: `brat status`

## Choosing a Workflow

| Goal | Approach |
|------|----------|
| Find and fix bugs | Use Mayor to analyze, then spawn agents |
| Implement a feature | Create convoy manually, add specific tasks |
| Review code | Use Mayor for analysis |
| Refactor | Create convoy with targeted tasks |
| Quick single task | Use `--solo` flag |

## Getting Help

- [Troubleshooting](troubleshooting.md) - Common issues
- [CLI Reference](../reference/cli.md) - All commands
- [Configuration](../configuration/index.md) - Customize behavior
