# AI Analysis

towl can use an LLM (Claude or any OpenAI-compatible model) to validate whether each TODO is still relevant. The `--ai` flag triggers analysis that determines if a TODO is **Valid**, **Invalid**, or **Uncertain**.

## Setup

Set your API key as an environment variable:

```bash
# Claude (default)
export TOWL_LLM_API_KEY=sk-ant-your-key-here

# Or for OpenAI
export TOWL_LLM_API_KEY=sk-your-openai-key
export TOWL_LLM_PROVIDER=openai
```

The API key is stored as a `SecretString` and never written to config files or logs.

## Basic Usage

```bash
# Non-interactive: analyse and filter out invalid TODOs
towl scan -N --ai

# Interactive: analyse and show results in TUI
towl scan --ai

# Combine with other flags
towl scan -N --ai -t fixme -f json -o fixmes.json
towl scan -N --ai -g  # create GitHub issues for valid TODOs only
```

## How It Works

For each TODO, the LLM receives:

1. **TODO description** -- the comment text
2. **Expanded context** -- ~30 lines of surrounding source code
3. **Function body** -- the complete enclosing function (if detected)

The LLM determines:

- **Is it resolved?** -- Does the code already do what the TODO asks?
- **Is it relevant?** -- Does the code/feature still exist?
- **Is it actionable?** -- Is the TODO clear and specific?

Based on these checks, each TODO is classified as Valid, Invalid, or Uncertain with a confidence score (0-100%).

## Non-Interactive Mode

With `-N --ai`, invalid TODOs are automatically filtered out of the results:

```bash
towl scan -N --ai
# Only valid and uncertain TODOs appear in output

towl scan -N --ai -g
# GitHub issues created only for valid TODOs, enriched with AI reasoning
```

## Interactive Mode (TUI)

With `--ai` (no `-N`), the TUI shows analysis results:

- **Validity column** -- Each TODO shows `V` (Valid), `I` (Invalid), or `?` (Uncertain)
- **Colour coding** -- Green for valid, red for invalid, yellow for uncertain
- **Peek view** -- Press `p` to see the LLM's reasoning below the source code
- **Delete invalid TODOs** -- Select invalid TODOs and press `d` to remove them from source files (with confirmation)

### Delete Workflow

1. Select invalid TODOs with `Space` (or `a` to select all visible)
2. Press `d` to open the delete confirmation dialog
3. Review the list of TODOs that will be removed
4. Press `y` to confirm deletion, or `n` to cancel
5. towl removes the comment lines from source files using atomic writes

> **Note:** Only TODOs marked as Invalid by the AI can be deleted via `d`. Valid and Uncertain TODOs are excluded from deletion.

## GitHub Issue Enrichment

When creating GitHub issues (either with `-g` or via the TUI), valid TODOs include an **AI Analysis** section in the issue body:

```markdown
## AI Analysis

**Validity:** Valid
**Confidence:** 92%

### Reasoning

The caching layer referenced in this TODO has not been implemented.
The function currently makes direct database calls on every request.

### Enhanced Description

This TODO identifies a performance bottleneck where database queries
are executed on every request without caching. Adding a caching layer
would reduce database load and improve response times.
```

## Configuration

Add a `[llm]` section to `.towl.toml`:

```toml
[llm]
provider = "claude"                      # "claude" or "openai"
model = "claude-opus-4-6"             # model identifier
# base_url = "http://localhost:11434/v1"  # for Ollama/vLLM
max_concurrent_analyses = 5              # concurrent LLM requests
max_analyse_count = 50                   # max TODOs to analyse per scan
max_tokens = 4096                        # LLM response token limit
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TOWL_LLM_API_KEY` | -- | API key (required for `--ai`) |
| `TOWL_LLM_PROVIDER` | `claude` | `"claude"`, `"openai"`, `"claude-code"`, or `"codex"` |
| `TOWL_LLM_MODEL` | `claude-opus-4-6` | Model identifier |
| `TOWL_LLM_BASE_URL` | Provider default | Custom endpoint URL |

### Using Claude Code or Codex CLI

If you have `claude` (Claude Code) or `codex` (OpenAI Codex CLI) installed, you can use them directly without an API key:

```bash
# Use Claude Code CLI
export TOWL_LLM_PROVIDER=claude-code
towl scan --ai

# Use Codex CLI
export TOWL_LLM_PROVIDER=codex
towl scan --ai
```

Or set in `.towl.toml`:

```toml
[llm]
provider = "claude-code"   # or "codex"
# command = "/custom/path/to/claude"   # optional override
# args = ["-p", "--output-format", "json"]  # optional override
```

No `TOWL_LLM_API_KEY` is needed -- the CLI agents manage their own authentication.

**Auto-fallback:** If the CLI binary is not found on PATH, towl automatically falls back to the corresponding API provider (`claude-code` -> Claude API, `codex` -> OpenAI API). The API fallback requires `TOWL_LLM_API_KEY` to be set.

### Using with Ollama or Local Models

```bash
export TOWL_LLM_PROVIDER=openai
export TOWL_LLM_MODEL=llama3
export TOWL_LLM_BASE_URL=http://localhost:11434/v1
export TOWL_LLM_API_KEY=ollama  # Ollama doesn't need a real key

towl scan -N --ai
```

## Rate Limiting

Two configurable limits prevent excessive API usage:

| Limit | Default | Config field |
|-------|---------|-------------|
| Concurrent requests | 5 | `max_concurrent_analyses` |
| Total TODOs analysed | 50 | `max_analyse_count` |

When the TODO count exceeds `max_analyse_count`, only the first N TODOs are analysed. A warning is logged for the remainder.
