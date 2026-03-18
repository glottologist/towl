# LLM

The LLM module provides AI-powered TODO validation using Claude (Anthropic API) or any OpenAI-compatible endpoint.

## `LlmProvider`

```rust
pub enum LlmProvider {
    Claude(ClaudeProvider),
    OpenAi(OpenAiProvider),
}
```

Dispatches LLM calls to the configured provider. Follows towl's existing enum dispatch pattern (`FormatterImpl`, `WriterImpl`).

### `call_raw`

```rust
pub async fn call_raw(
    &self,
    user_content: &str,
    system_prompt: &str,
    api_key: &SecretString,
) -> Result<(String, LlmUsage), TowlLlmError>
```

Sends a prompt to the LLM and returns the response text and token usage.

### `build_provider`

```rust
pub fn build_provider(config: &LlmConfig) -> Result<LlmProvider, TowlLlmError>
```

Factory function that creates the appropriate provider from configuration.

## `analyse_todos`

```rust
pub async fn analyse_todos(
    todos: &mut [TodoComment],
    config: &LlmConfig,
) -> Result<AnalysisSummary, TowlLlmError>
```

Main entry point for TODO analysis. For each TODO (up to `max_analyse_count`):

1. Reads expanded context (~30 lines around the TODO + full function body)
2. Constructs a prompt with the TODO description, file path, and code context
3. Calls the LLM to determine validity
4. Parses the structured JSON response into an `AnalysisResult`
5. Attaches the result to `TodoComment.analysis`

Concurrency is bounded by `max_concurrent_analyses` via a tokio `Semaphore`.

**Errors:**

- `NotConfigured` -- `TOWL_LLM_API_KEY` not set
- `UnsupportedProvider` -- Provider is not "claude" or "openai"
- `ApiError`, `AuthError`, `RateLimited` -- From the LLM API

## `Validity`

```rust
pub enum Validity {
    Valid,
    Invalid,
    Uncertain,
}
```

Whether a TODO is still valid:

| Value | Meaning |
|-------|---------|
| `Valid` | TODO describes work that still needs to be done |
| `Invalid` | TODO has been resolved, is irrelevant, or is nonsensical |
| `Uncertain` | Cannot determine validity from available context |

## `AnalysisResult`

```rust
pub struct AnalysisResult {
    pub validity: Validity,
    pub reasoning: String,
    pub is_resolved: bool,
    pub is_relevant: bool,
    pub is_actionable: bool,
    pub confidence: f64,
    pub enrichment: String,
}
```

| Field | Description |
|-------|-------------|
| `validity` | Overall assessment |
| `reasoning` | Explanation of why the TODO is valid/invalid/uncertain |
| `is_resolved` | Whether the code already implements what the TODO asks |
| `is_relevant` | Whether the code/feature the TODO references still exists |
| `is_actionable` | Whether the TODO describes a clear, specific task |
| `confidence` | 0.0-1.0 confidence in the assessment |
| `enrichment` | Enhanced description suitable for a GitHub issue body |

## `AnalysisSummary`

```rust
pub struct AnalysisSummary {
    pub valid_count: usize,
    pub invalid_count: usize,
    pub uncertain_count: usize,
    pub error_count: usize,
}
```

Summary counts returned by `analyse_todos()`.

## Providers

### `ClaudeProvider`

POST to `https://api.anthropic.com/v1/messages` with headers:
- `x-api-key`: API key
- `anthropic-version`: `2023-06-01`

System prompt is a top-level `system` field (not in the messages array).

### `OpenAiProvider`

POST to `{base_url}/chat/completions` with `Authorization: Bearer {key}`.
Default base URL: `https://api.openai.com/v1`. Configurable for Ollama, vLLM, etc.

System prompt is the first message in the `messages` array with `role: "system"`.

### `ClaudeCodeProvider`

Invokes the `claude` CLI as a subprocess with `-p --output-format json`. The combined system prompt and user content are passed as the final argument. No API key required.

Default command: `claude`. Configurable via `llm.command` and `llm.args`.

Auto-falls back to `ClaudeProvider` (API) if the CLI binary is not found on PATH.

### `CodexProvider`

Invokes the `codex` CLI as a subprocess with `-q`. The combined prompt is passed as the final argument. No API key required.

Default command: `codex`. Configurable via `llm.command` and `llm.args`.

Auto-falls back to `OpenAiProvider` (API) with `gpt-4o` if the CLI binary is not found on PATH.

### `is_cli_provider`

```rust
pub const fn is_cli_provider(&self) -> bool
```

Returns `true` for `ClaudeCode` and `Codex` variants. Used to skip the API key requirement for CLI-based providers.

## Configuration

See [Configuration](../getting-started/configuration.md#llm-section) for the `[llm]` config section.

## Errors

```rust
pub enum TowlLlmError {
    ApiError { message, status },
    AuthError,
    RateLimited { retry_after_secs },
    ParseError { message },
    NotConfigured,
    UnsupportedProvider { provider },
    AnalysisLimitExceeded { count, max },
    HttpError(String),
}
```

| Variant | Cause |
|---------|-------|
| `ApiError` | LLM API returned a non-200 status |
| `AuthError` | 401 -- invalid or missing API key |
| `RateLimited` | 429 -- too many requests |
| `ParseError` | LLM response could not be parsed as valid JSON |
| `NotConfigured` | `TOWL_LLM_API_KEY` environment variable not set |
| `UnsupportedProvider` | Provider is not "claude" or "openai" |
| `AnalysisLimitExceeded` | TODO count exceeds `max_analyse_count` |
| `HttpError` | HTTP client or file I/O error |
