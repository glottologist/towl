# Security

towl applies defence-in-depth across configuration, scanning, and output.

## Path Traversal Protection

All user-supplied paths are checked for `..` components before use:

- **Config paths** -- `towl init --path` rejects traversal attempts
- **Scan paths** -- `towl scan <path>` validates before walking
- **Output paths** -- `-o <path>` is validated and symlinks are resolved

The check uses `contains_path_traversal()` which inspects each path component for `..`.

## Symlink Resolution

Output file paths are resolved via `std::fs::canonicalize()` before writing. This prevents symlink-based escape from the intended output directory.

## Resource Limits

Hard limits prevent denial-of-service via large repositories or malicious inputs:

| Limit | Value | Purpose |
|-------|-------|---------|
| Max file size | 10 MB | Prevents reading huge binary/generated files |
| Max TODOs per file | 10,000 | Bounds per-file memory usage |
| Max total TODOs | 100,000 | Bounds overall memory usage |
| Max files scanned | 100,000 | Bounds directory walk |
| Max pattern length | 256 chars | Prevents regex DoS via long patterns |
| Max compiled regex | 256 KB | Bounds regex engine memory |
| Max total patterns (combined) | 50 | Bounds total regex compilation across all categories |
| Max patterns per config field | 100 | Limits config file attack surface |

## Secret Handling

The GitHub token (`TOWL_GITHUB_TOKEN`) is:

- **Never stored in config files** -- Only accepted via environment variable
- **Stored as `SecretString`** -- Uses the `secrecy` crate
- **Masked in debug output** -- `Debug` and `Display` show `[REDACTED]`
- **Zeroed on drop** -- Memory is cleared when the config is dropped

## Environment Variable Restriction

Only three environment variables are read:

| Variable | Purpose |
|----------|---------|
| `TOWL_GITHUB_TOKEN` | GitHub authentication |
| `TOWL_GITHUB_OWNER` | Repository owner override |
| `TOWL_GITHUB_REPO` | Repository name override |

No other environment variables influence behaviour.

## Config File Safety

- **`--force` required** for overwriting existing config files
- **Pattern array limits** -- Each pattern field is capped at 100 entries
- **Pattern length limits** -- Individual regex patterns capped at 256 characters
- **TOML parsing** -- Uses `config` crate with `serde` for type-safe deserialization

## Git Integration

- Git operations use `tokio::process::Command` to run `git` as a subprocess
- Only read-only git commands are executed (`git remote get-url origin`)
- No git credentials are accessed or stored

## .gitignore Respect

The `ignore` crate automatically respects `.gitignore` rules during directory walking, preventing scanning of files the user has excluded from version control.

## Error Messages

Error messages include file paths and context for debugging but do not expose internal implementation details or sensitive data. The `SecretString` type ensures tokens cannot leak through error formatting.

## Threat Model

| Threat | Mitigation |
|--------|------------|
| Path traversal via config/scan/output paths | `..` component detection, symlink resolution |
| Regex DoS via malicious patterns | Pattern length limit (256), regex size limit (256 KB), total pattern cap (50) |
| Memory exhaustion via large repos | File size, TODO count, and file count limits |
| Token leakage | `SecretString`, env-only token, masked debug |
| Config file overwrite | `--force` flag required |
| Arbitrary file write via symlinks | `canonicalize()` on output paths |
| Scanning outside intended directory | `.gitignore` respect, extension filtering |
