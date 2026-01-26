# Contributing to Brat

Thank you for your interest in contributing to Brat! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

### Prerequisites

- **Rust** (stable, 1.75+): Install via [rustup](https://rustup.rs/)
- **Node.js** (20+): For the web UI
- **Git**: Version 2.30+
- **Grite**: The substrate for brat - see [grite repository](https://github.com/anthropics/grite)

### Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/neul-labs/brat.git
   cd brat
   ```

2. **Build the Rust components**
   ```bash
   cargo build --release
   ```

3. **Set up the web UI**
   ```bash
   cd brat-ui
   npm install
   npm run dev
   ```

4. **Run tests**
   ```bash
   cargo test
   cd brat-ui && npm run build
   ```

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates. When creating a bug report, include:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected behavior vs actual behavior
- Your environment (OS, Rust version, Node version)
- Relevant logs or error messages

### Suggesting Features

Feature suggestions are welcome! Please:

- Check existing issues and discussions first
- Provide a clear use case
- Explain why this feature would be useful to most users
- Consider how it fits with the project's goals

### Code Contributions

1. **Find an issue to work on** - Look for issues labeled `good first issue` or `help wanted`
2. **Comment on the issue** - Let others know you're working on it
3. **Fork and branch** - Create a feature branch from `main`
4. **Make your changes** - Follow the coding standards below
5. **Test your changes** - Ensure all tests pass
6. **Submit a pull request** - Reference the issue in your PR

## Pull Request Process

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make atomic commits** with clear messages:
   ```
   feat: add convoy status filtering

   - Add status filter to convoy list endpoint
   - Update UI to show filter dropdown
   - Add tests for new functionality
   ```

3. **Keep PRs focused** - One feature or fix per PR

4. **Update documentation** if needed

5. **Ensure CI passes** - All tests and lints must pass

6. **Request review** - Tag maintainers for review

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

## Coding Standards

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Use meaningful variable and function names
- Add doc comments for public APIs

```rust
/// Creates a new convoy with the given title.
///
/// # Arguments
///
/// * `title` - The convoy title (must be non-empty)
/// * `body` - Optional description
///
/// # Errors
///
/// Returns an error if the title is empty or if grite operations fail.
pub fn create_convoy(title: &str, body: Option<&str>) -> Result<Convoy> {
    // ...
}
```

### TypeScript/Vue

- Use TypeScript for all new code
- Follow Vue 3 Composition API patterns
- Use `<script setup lang="ts">` syntax
- Run `npm run build` to check for type errors

### General

- Keep functions small and focused
- Prefer explicit over implicit
- Handle errors gracefully
- Avoid premature optimization

## Testing

### Rust Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p libbrat-engine

# Run with output
cargo test -- --nocapture
```

### UI Tests

```bash
cd brat-ui
npm run build  # Type checking
npm run lint   # Linting (if configured)
```

### Writing Tests

- Write tests for new functionality
- Include edge cases and error conditions
- Use descriptive test names

```rust
#[test]
fn create_convoy_with_empty_title_fails() {
    let result = create_convoy("", None);
    assert!(result.is_err());
}
```

## Documentation

- Update README.md for user-facing changes
- Add doc comments to public Rust APIs
- Update `docs/` for architectural changes
- Include code examples where helpful

## Project Structure

```
brat/
├── crates/
│   ├── brat/           # Main CLI binary
│   ├── libbrat-engine/ # Core engine logic
│   ├── libbrat-grite/   # Grite integration
│   └── libbrat-ipc/    # IPC protocols
├── brat-ui/            # Vue.js web dashboard
├── docs/               # Architecture documentation
└── scripts/            # Helper scripts
```

## Questions?

- Open a [Discussion](https://github.com/neul-labs/brat/discussions) for questions
- Check existing [Issues](https://github.com/neul-labs/brat/issues) for known problems
- Read the [Documentation](docs/) for architecture details

## License

By contributing to Brat, you agree that your contributions will be licensed under the MIT License.
