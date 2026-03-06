# Installation

## From crates.io

```bash
cargo install towl
```

Requires Rust 1.75 or later. Install Rust via [rustup](https://rustup.rs/) if needed.

## From Source

```bash
git clone https://github.com/glottologist/towl.git
cd towl
cargo build --release
```

The binary will be at `target/release/towl`.

## Verify Installation

```bash
towl --version
towl --help
```

## Requirements

- **Rust**: 1.75+
- **git**: Required on `PATH` for `towl init` (extracts GitHub owner/repo from the git remote)
- **OS**: Linux, macOS, Windows
