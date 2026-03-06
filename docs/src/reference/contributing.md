# Contributing

## Getting Started

```bash
git clone https://github.com/glottologist/towl.git
cd towl
cargo build
```

### Requirements

- **Rust** 1.75+ (see `rust-toolchain.toml`)
- **git** on `PATH`

## Development Commands

```bash
# Build
cargo build

# Run all tests
cargo nextest run   # preferred
cargo test          # fallback

# Clippy (strict)
cargo clippy --all-targets --all-features

# Format
cargo fmt

# Run the binary
cargo run -- scan
cargo run -- scan -f json -o todos.json
cargo run -- config
cargo run -- init
```

## Project Structure

See [Architecture](./architecture.md) for a full layout. Key entry points:

- `src/bin/towl.rs` -- CLI binary
- `src/lib/mod.rs` -- Library root
- `tests/` -- Integration and property-based tests

## Testing

### Test Hierarchy

Tests follow a strict hierarchy:

1. **proptest** (property-based) -- First choice for pure functions, parsers, validators, serialisation roundtrips
2. **rstest** (parameterized) -- For specific known cases (< 10 inputs with exact expected outputs)
3. **Standalone** -- Last resort, for complex integration scenarios

### Running Tests

```bash
# All tests
cargo nextest run

# Specific module
cargo nextest run scanner

# Property-based tests only
cargo nextest run proptest

# Integration tests only
cargo nextest run --test '*'
```

## Code Style

- Follow Rust naming conventions (`snake_case` for functions, `CamelCase` for types)
- All public items need doc comments (`///`)
- No `#[allow(...)]` attributes -- fix the underlying issue
- No `.unwrap()` / `.expect()` in production code -- use `?` with typed errors
- No `as` numeric casts -- use `try_from` / `into` / `From`
- Minimise `.clone()` -- prefer borrowing, see Clone Reduction Policy

## Error Handling

- Use `thiserror` for error type derivation
- Each module defines its own error enum
- Errors propagate upward via `?` and `#[from]`
- Never silently discard `Result` values

## Adding a New Output Format

1. Create `src/lib/output/formatter/formatters/yourformat.rs`
2. Implement the `Formatter` trait
3. Add a variant to `FormatterImpl` in `formatters/mod.rs`
4. Add dispatch in `FormatterImpl::format()`
5. Add a variant to `OutputFormat` in `src/lib/cli/mod.rs`
6. Update the format-to-writer mapping in `Output::new()`
7. Add tests (proptest for roundtrips, rstest for edge cases)

## Adding a New TODO Type

1. Add a variant to `TodoType` in `src/lib/comment/todo.rs`
2. Update `Display`, `TryFrom<&str>`, `as_filter_str()`
3. Add a default pattern to `default_todo_patterns()` in `src/lib/config/types.rs`
4. Add a pattern mapping in the parser
5. Update tests

## Pull Requests

- Keep PRs focused on a single change
- Include tests for new functionality
- Ensure `cargo clippy` passes with zero warnings
- Ensure `cargo fmt` produces no changes
- Ensure all tests pass
