# Processor

The processor replaces TODO comments in source files with GitHub issue links after issues are created.

## `Processor`

```rust
pub struct Processor;
```

Stateless processor that operates on batches of `(TodoComment, CreatedIssue)` pairs. All methods are associated functions (no `self`).

### `replace_todos`

```rust
pub async fn replace_todos(
    repo_root: &Path,
    replacements: &[(TodoComment, CreatedIssue)],
) -> ProcessorResult
```

Replaces TODO comments in source files with `GH_ISSUE: <issue_url>` links.

**Behaviour:**

1. Groups replacements by file path for efficient batch processing
2. For each file, validates the path stays within `repo_root`
3. Reads file content, replaces each TODO line, writes back atomically
4. Returns a `ProcessorResult` with counts and per-file errors

**Path safety:**

- Both the file path and repo root are canonicalised before comparison
- Files outside the repo root are rejected with `PathOutsideRoot`
- Issue URLs must start with `https://github.com/` or are rejected

**Replacement format:**

The comment prefix (e.g., `// `, `# `, `/* `) is preserved. The TODO text after the prefix is replaced:

```text
// TODO: Implement caching    -->    // GH_ISSUE: https://github.com/owner/repo/issues/42
# FIXME: Handle timeout        -->    # GH_ISSUE: https://github.com/owner/repo/issues/43
```

**Atomic writes:**

Files are written via a tempfile in the same directory, then atomically persisted. This prevents partial writes if the process is interrupted.

**Empty input:**

If `replacements` is empty, returns immediately with zero counts and no I/O.

## `ProcessorResult`

```rust
pub struct ProcessorResult {
    pub files_modified: usize,
    pub todos_replaced: usize,
    pub errors: Vec<(PathBuf, TowlProcessorError)>,
}
```

Summary of a batch replacement operation. The `errors` field contains per-file errors that did not abort the overall operation -- other files continue processing.

## Errors

```rust
pub enum TowlProcessorError {
    FileReadError(PathBuf, std::io::Error),
    FileWriteError(PathBuf, std::io::Error),
    LineOutOfBounds { path: PathBuf, line: usize, total_lines: usize },
    CommentPrefixNotFound { path: PathBuf, line: usize },
    PathOutsideRoot { path: PathBuf, root: PathBuf },
    InvalidIssueUrl { url: String },
}
```

| Variant | Cause |
|---------|-------|
| `FileReadError` | Failed to read source file |
| `FileWriteError` | Failed to write modified file |
| `LineOutOfBounds` | TODO line number exceeds file length |
| `CommentPrefixNotFound` | Column offset points past end of line |
| `PathOutsideRoot` | File is outside the repository root |
| `InvalidIssueUrl` | URL does not start with `https://github.com/` |

## Example

```rust,no_run
use towl::processor::Processor;

let replacements = vec![(todo, created_issue)];
let result = Processor::replace_todos(repo_root, &replacements).await;

println!("Modified {} files, replaced {} TODOs", result.files_modified, result.todos_replaced);

for (path, err) in &result.errors {
    eprintln!("Error in {}: {}", path.display(), err);
}
```
