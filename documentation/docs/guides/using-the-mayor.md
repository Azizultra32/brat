# Using the Mayor

The Mayor is Brat's AI orchestrator that analyzes codebases and creates work.

## Starting the Mayor

Start a Mayor session:

```bash
brat mayor start
```

The Mayor runs as a persistent session that you can interact with.

## Asking the Mayor

Send requests to the Mayor:

```bash
# Analyze code for bugs
brat mayor ask "Analyze src/ and identify any bugs"

# Create tasks for improvements
brat mayor ask "Review the authentication flow and suggest improvements"

# Plan a feature
brat mayor ask "Plan the implementation of a user dashboard"
```

## Common Requests

### Code Analysis

```bash
# Find bugs
brat mayor ask "Analyze the codebase for potential bugs"

# Security review
brat mayor ask "Identify security vulnerabilities in src/"

# Performance issues
brat mayor ask "Find performance bottlenecks in the API handlers"
```

### Creating Work

```bash
# Bug fix convoy
brat mayor ask "Create a convoy to fix the bugs you identified"

# Feature convoy
brat mayor ask "Create a convoy for implementing dark mode"

# Refactoring convoy
brat mayor ask "Create a convoy to refactor the database layer"
```

### Getting Information

```bash
# Explain code
brat mayor ask "Explain how the authentication flow works"

# Find dependencies
brat mayor ask "What components depend on the user service?"

# Assess complexity
brat mayor ask "Assess the complexity of refactoring the payment module"
```

## Mayor Outputs

The Mayor creates:

- **Convoys** - Groups of related work
- **Tasks** - Individual work items with titles, paths, and context
- **Comments** - Analysis and recommendations

View what the Mayor created:

```bash
brat status
brat convoy list
brat task list
```

## Checking Mayor Status

```bash
# View current Mayor session
brat mayor status

# View Mayor session details
brat session list --json | jq '.[] | select(.role == "mayor")'
```

## Stopping the Mayor

Stop the Mayor session:

```bash
brat mayor stop
```

## Best Practices

### Be Specific

**Good:** "Analyze src/services/user.rs for null pointer issues"
**Less good:** "Find bugs"

### Scope Your Requests

**Good:** "Create tasks for the top 5 bugs by severity"
**Less good:** "Create tasks for all issues"

### Iterate

1. Start with analysis: "Analyze the code"
2. Review results: `brat status`
3. Refine: "Focus on the security issues only"
4. Create work: "Create a convoy for the critical security fixes"

### Use for Planning

The Mayor excels at:

- Breaking down large features into tasks
- Prioritizing work
- Identifying dependencies between tasks

## Example Session

```bash
# Start the Mayor
$ brat mayor start
Mayor started (session: s-20250121-abc1)

# Analyze code
$ brat mayor ask "Analyze src/ for bugs and security issues"
Analyzing src/...

Found 3 issues:
1. [P0] SQL injection in user query (src/db/users.rs:42)
2. [P1] Unchecked null in login flow (src/auth/login.rs:78)
3. [P2] Potential race condition (src/cache/refresh.rs:120)

# Create a convoy
$ brat mayor ask "Create a convoy to fix these issues"
Created convoy: convoy-20250121-xyz9 "Security and bug fixes"
Created tasks:
- task-abc1: Fix SQL injection
- task-def2: Fix null check
- task-ghi3: Fix race condition

# View status
$ brat status
Convoys:
  convoy-20250121-xyz9 "Security and bug fixes" (3 tasks)

Tasks:
  task-abc1 "Fix SQL injection" [queued] P0
  task-def2 "Fix null check" [queued] P1
  task-ghi3 "Fix race condition" [queued] P2

# Stop Mayor (optional)
$ brat mayor stop
Mayor stopped
```

## Troubleshooting

### Mayor Not Responding

Check if the Mayor session is active:

```bash
brat mayor status
brat session list
```

Restart if needed:

```bash
brat mayor stop
brat mayor start
```

### Analysis Taking Too Long

Scope your request to specific paths:

```bash
# Instead of:
brat mayor ask "Analyze the entire codebase"

# Try:
brat mayor ask "Analyze src/services/ for bugs"
```

### Mayor Creating Too Many Tasks

Be more specific about what you want:

```bash
brat mayor ask "Create a convoy with at most 5 tasks for the critical issues"
```
