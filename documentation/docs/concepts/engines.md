# AI Engines

Brat works with multiple AI coding tools through a unified engine interface.

## Supported Engines

| Engine | Command | Best For |
|--------|---------|----------|
| **Claude Code** | `claude` | Native Anthropic integration, session resume |
| **Aider** | `aider` | Multi-model flexibility, local LLM support |
| **OpenCode** | `opencode` | 75+ providers, Claude Code alternative |
| **Codex** | `codex` | Structured JSON output |
| **Continue** | `cn` | IDE integration, CI/CD |
| **Gemini** | `gemini` | Google's free tier |
| **GitHub Copilot** | `gh copilot` | Shell/git suggestions |

## Engine Configuration

Set your default engine in `.brat/config.toml`:

```toml
[engine]
default = "claude"

[engine.claude]
# Claude Code specific settings

[engine.aider]
model = "gpt-4"
# Aider specific settings
```

## Engine Interface

All engines implement a common interface:

```rust
trait Engine {
  fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult>;
  fn send(&self, session: SessionHandle, input: EngineInput) -> Result<()>;
  fn tail(&self, session: SessionHandle, n: usize) -> Result<Vec<String>>;
  fn stop(&self, session: SessionHandle, how: StopMode) -> Result<()>;
  fn health(&self, session: SessionHandle) -> Result<EngineHealth>;
}
```

This means:

- Any engine can work with any Brat workflow
- New engines can be added by implementing the trait
- Engine-specific quirks are hidden behind the abstraction

## Engine Comparison

### Claude Code

```toml
[engine.claude]
# Default Claude Code settings
```

**Pros:**

- Native Anthropic integration
- Session resume capability
- Best context understanding

**Cons:**

- Requires Anthropic API key
- Usage-based pricing

### Aider

```toml
[engine.aider]
model = "gpt-4"
# Or use Claude
model = "claude-3-opus"
```

**Pros:**

- Multi-model support (GPT-4, Claude, Gemini, local)
- Works with local LLMs
- Active open-source community

**Cons:**

- Requires separate LLM API key
- More configuration needed

### OpenCode

```toml
[engine.opencode]
provider = "anthropic"
```

**Pros:**

- 75+ LLM providers
- Open-source Claude Code alternative
- Flexible configuration

**Cons:**

- Newer project
- Less documentation

### Codex

```toml
[engine.codex]
# Codex settings
```

**Pros:**

- Structured JSON output
- Easy to parse responses

**Cons:**

- Older technology
- Limited context window

## Timeouts and Safety

All engine operations have bounded timeouts:

| Operation | Default Timeout |
|-----------|----------------|
| Spawn | 60 seconds |
| Send | 5 seconds |
| Tail | 10 seconds |
| Stop | 10 seconds |
| Health | 5 seconds |

Configure in `.brat/config.toml`:

```toml
[engine]
spawn_timeout = 120  # seconds
send_timeout = 10
```

## Engine Selection

### Per-Task Override

Specify an engine for a specific task:

```bash
brat task add \
  --convoy <id> \
  --title "Complex refactor" \
  --engine aider
```

### Fallback Chain

Configure fallback engines:

```toml
[engine]
default = "claude"
fallback = ["aider", "opencode"]
```

If Claude fails, Brat tries Aider, then OpenCode.

## Setting Up Engines

### Claude Code

1. Install Claude Code:
   ```bash
   npm install -g @anthropic-ai/claude-code
   ```

2. Configure API key:
   ```bash
   export ANTHROPIC_API_KEY=your-key
   ```

3. Set as default:
   ```toml
   [engine]
   default = "claude"
   ```

### Aider

1. Install Aider:
   ```bash
   pip install aider-chat
   ```

2. Configure model:
   ```toml
   [engine.aider]
   model = "gpt-4"
   ```

3. Set API key:
   ```bash
   export OPENAI_API_KEY=your-key
   ```

### OpenCode

1. Install OpenCode:
   ```bash
   cargo install opencode
   ```

2. Configure provider:
   ```toml
   [engine.opencode]
   provider = "anthropic"
   model = "claude-3-opus"
   ```

## Engine Health

Check engine availability:

```bash
# View session health
brat session list --json | jq '.[].health'
```

The Witness monitors engine health and restarts unhealthy sessions.

## Adding Custom Engines

Implement the `Engine` trait for custom integrations:

1. Create a new crate: `libbrat-engine-myengine`
2. Implement the trait methods
3. Register in `libbrat-engine/src/lib.rs`
4. Add configuration options

See the existing engine implementations for examples.
