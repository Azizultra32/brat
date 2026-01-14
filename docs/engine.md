# Engine abstraction

Engines encapsulate how sessions are spawned and controlled (Claude Code, Codex CLI, OpenCode, shell, etc). The harness uses engine sessions and records progress in Grit.

## Trait

```rust
trait Engine {
  fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult>;
  fn send(&self, session: SessionHandle, input: EngineInput) -> Result<()>;
  fn tail(&self, session: SessionHandle, n: usize) -> Result<Vec<String>>;
  fn stop(&self, session: SessionHandle, how: StopMode) -> Result<()>;
  fn health(&self, session: SessionHandle) -> Result<EngineHealth>;
}
```

## Implementations

- `engine-claude-code`
- `engine-codex-cli`
- `engine-opencode`
- `engine-shell` (tests/simulation)

## Safety

- All engine calls are wrapped in bounded timeouts.
- Outputs are captured and hashed for reference.
- Errors are structured and surfaced via Grit comments or labels.
