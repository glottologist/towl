# GitHub

The GitHub module creates issues from TODO comments, detects duplicates, and handles rate limiting.

## `GitHubClient`

```rust
pub struct GitHubClient {
    // private fields
}
```

Authenticated GitHub API client for creating issues from TODO comments. Maintains a cache of existing issue titles and TODO IDs for deduplication. Includes rate-limit handling with configurable delays and automatic retries.

### `new`

```rust
pub fn new(config: &GitHubConfig) -> Result<Self, TowlGitHubError>
```

Creates a new client from a `GitHubConfig`. Exposes the `SecretString` token once to build the Octocrab API client.

**Errors:**

- `MissingToken` -- Token is empty
- `ApiError` -- Octocrab client failed to build

### `load_existing_issues`

```rust
pub async fn load_existing_issues(&mut self) -> Result<(), TowlGitHubError>
```

Paginates through all existing issues (open and closed) in the repository, caching their titles and embedded TODO IDs. Call this before `create_issue` to enable duplicate detection.

**Errors:**

- `ApiError` -- GitHub API call failed
- `AuthError` -- Invalid or expired token
- `RepositoryNotFound` -- Owner/repo combination does not exist

### `issue_exists`

```rust
pub fn issue_exists(&self, todo: &TodoComment) -> bool
```

Returns `true` if a matching issue already exists, checked by TODO ID (embedded in issue body) or by generated title.

### `create_issue`

```rust
pub async fn create_issue(
    &mut self,
    todo: &TodoComment,
) -> Result<CreatedIssue, TowlGitHubError>
```

Creates a GitHub issue for a TODO comment. Generates a title with type prefix, truncated description, and file location. The body includes file path, line number, column range, description, function context, original comment, and surrounding code.

Automatically retries on rate limiting (up to 3 attempts).

**Errors:**

- `IssueAlreadyExists` -- Duplicate detected
- `RateLimitExceeded` -- Rate limit hit after max retries
- `ApiError` -- GitHub API failure
- `AuthError` -- Authentication failure

### Issue Title Format

```text
[TODO] Implement caching (cache.rs:42)
```

Titles are capped at 50 characters (excluding the type prefix and location suffix). Long descriptions are truncated at word boundaries with `...`.

### Issue Body Sections

1. **TODO Details** -- Type, file, line, column range
2. **Description** -- Extracted description text (Markdown-escaped)
3. **Function Context** -- Enclosing function name (if detected)
4. **Original Comment** -- Full comment line in a code block
5. **Context** -- Surrounding source lines in a code block
6. **TODO ID** -- Embedded identifier for deduplication

### Duplicate Detection

Issues are deduplicated by two methods:

1. **TODO ID** -- Each issue body contains `*TODO ID: {file_path}_L{line_number}*`. If any existing issue body contains the same ID, the TODO is skipped.
2. **Title match** -- If the generated title matches an existing issue title exactly, the TODO is skipped.

## `CreatedIssue`

```rust
pub struct CreatedIssue {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub todo_id: String,
}
```

Metadata for a successfully created GitHub issue. Implements `Serialize` and `Deserialize` for JSON roundtripping.

## Errors

```rust
pub enum TowlGitHubError {
    ApiError { message: String, source: Option<octocrab::Error> },
    AuthError,
    RateLimitExceeded { retry_after_secs: u64 },
    IssueAlreadyExists { title: String },
    RepositoryNotFound { owner: String, repo: String },
    MissingToken,
}
```

| Variant | Cause |
|---------|-------|
| `ApiError` | General GitHub API failure |
| `AuthError` | 401 response -- invalid or expired token |
| `RateLimitExceeded` | 403 with "rate limit" in message |
| `IssueAlreadyExists` | Duplicate detected before creation |
| `RepositoryNotFound` | 404 response -- owner/repo not found |
| `MissingToken` | `TOWL_GITHUB_TOKEN` not set or empty |

## Example

```rust,no_run
use towl::config::TowlConfig;
use towl::github::GitHubClient;

let config = TowlConfig::load(None)?;
let mut client = GitHubClient::new(&config.github)?;

// Load existing issues for duplicate detection
client.load_existing_issues().await?;

// Create an issue (skips if duplicate)
if !client.issue_exists(&todo) {
    let issue = client.create_issue(&todo).await?;
    println!("Created #{}: {}", issue.number, issue.html_url);
}
```
