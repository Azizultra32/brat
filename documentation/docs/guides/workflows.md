# Workflow Templates

Workflow templates define reusable patterns for common tasks.

## What Are Workflows?

Workflows are YAML files in `.brat/workflows/` that define:

- **Steps** - Sequential or parallel tasks
- **Dependencies** - Which steps depend on others
- **Variables** - Parameterized values

## Workflow Location

```
.brat/
└── workflows/
    ├── feature.yaml
    ├── fix-bug.yaml
    └── code-review.yaml
```

## Sequential Workflow

Steps that run one after another:

```yaml
# .brat/workflows/feature.yaml
name: feature
type: workflow
steps:
  - id: design
    title: "Design {{feature}}"

  - id: implement
    needs: [design]
    title: "Implement {{feature}}"

  - id: test
    needs: [implement]
    title: "Test {{feature}}"
```

### Using a Sequential Workflow

```bash
brat convoy create --workflow feature --var feature="user dashboard"
```

Creates tasks:

1. "Design user dashboard"
2. "Implement user dashboard" (waits for design)
3. "Test user dashboard" (waits for implement)

## Parallel Workflow (Convoy)

Tasks that run in parallel:

```yaml
# .brat/workflows/code-review.yaml
name: code-review
type: convoy
legs:
  - id: correctness
    title: "Review correctness"

  - id: security
    title: "Review security"

  - id: performance
    title: "Review performance"

synthesis:
  title: "Synthesize review findings"
```

### Using a Parallel Workflow

```bash
brat convoy create --workflow code-review
```

Creates:

- 3 parallel review tasks
- 1 synthesis task that waits for all reviews

## Workflow Syntax

### Basic Structure

```yaml
name: workflow-name
type: workflow | convoy
steps: []    # for type: workflow
legs: []     # for type: convoy
synthesis:   # for type: convoy
```

### Steps (Sequential)

```yaml
steps:
  - id: unique-id
    title: "Task title"
    paths: ["src/path.rs"]     # optional
    priority: P1               # optional
    needs: [other-step-id]     # optional dependencies
```

### Legs (Parallel)

```yaml
legs:
  - id: unique-id
    title: "Task title"
    paths: ["src/path.rs"]     # optional
```

### Synthesis

Runs after all legs complete:

```yaml
synthesis:
  title: "Combine results"
  paths: ["output/"]
```

### Variables

Use `{{variable}}` syntax:

```yaml
title: "Fix {{bug_type}} in {{component}}"
paths: ["src/{{component}}/"]
```

Pass variables when creating:

```bash
brat convoy create --workflow fix-bug \
  --var bug_type="memory leak" \
  --var component="cache"
```

## Example Workflows

### Bug Fix Workflow

```yaml
# .brat/workflows/fix-bug.yaml
name: fix-bug
type: workflow
steps:
  - id: investigate
    title: "Investigate {{issue}}"
    priority: P1

  - id: fix
    needs: [investigate]
    title: "Fix {{issue}}"

  - id: test
    needs: [fix]
    title: "Test fix for {{issue}}"

  - id: document
    needs: [test]
    title: "Document fix for {{issue}}"
    priority: P2
```

### Feature Development

```yaml
# .brat/workflows/feature.yaml
name: feature
type: workflow
steps:
  - id: design
    title: "Design {{feature}}"
    paths: ["docs/design/"]

  - id: implement
    needs: [design]
    title: "Implement {{feature}}"
    paths: ["src/"]

  - id: test
    needs: [implement]
    title: "Add tests for {{feature}}"
    paths: ["tests/"]

  - id: document
    needs: [implement]
    title: "Document {{feature}}"
    paths: ["docs/"]
```

### Multi-Reviewer Code Review

```yaml
# .brat/workflows/review.yaml
name: review
type: convoy
legs:
  - id: review-1
    title: "Code review - correctness"

  - id: review-2
    title: "Code review - security"

  - id: review-3
    title: "Code review - maintainability"

synthesis:
  title: "Consolidate review feedback"
```

## Creating Workflows

1. Create the workflow file:
   ```bash
   mkdir -p .brat/workflows
   ```

2. Write the YAML definition

3. Test with a dry run (if supported):
   ```bash
   brat convoy create --workflow feature --var feature="test" --dry-run
   ```

4. Create the actual convoy:
   ```bash
   brat convoy create --workflow feature --var feature="my feature"
   ```

## Best Practices

### Keep Workflows Focused

Each workflow should handle one type of work:

- `feature.yaml` - New features
- `fix-bug.yaml` - Bug fixes
- `refactor.yaml` - Refactoring

### Use Descriptive IDs

```yaml
# Good
- id: implement-api
- id: add-tests

# Less clear
- id: step1
- id: step2
```

### Document Variables

Add a comment at the top:

```yaml
# Variables:
#   feature: Name of the feature to implement
#   component: Target component (e.g., "auth", "api")
name: feature
```

### Test Before Using

Create a test convoy to verify the workflow:

```bash
brat convoy create --workflow feature --var feature="test"
brat convoy show <convoy-id>  # Verify structure
```
