# Engine abstraction

Engines encapsulate how sessions are spawned and controlled (Claude Code, Codex CLI, OpenCode, shell, etc). The harness uses engine sessions and records progress in Grite.

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
- Errors are structured and surfaced via Grite comments or labels.

## Normalization defaults

- Spawn timeout: 60s
- Send timeout: 5s
- Tail timeout: 10s
- Stop timeout: 10s
- Health timeout: 5s
- Spawn retry: 1 (with backoff)

Exit codes are normalized to `exit_code` and `exit_reason` in session comments.
