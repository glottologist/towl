# Parser

The parser reads file content, identifies comment lines using regex patterns, extracts TODO items, and captures surrounding context.

## `Parser`

```rust
pub struct Parser {
    comment_patterns: Vec<Regex>,
    patterns: Vec<Pattern>,
    function_patterns: Vec<Regex>,
    context_lines: usize,
}
```

The parser is `pub(crate)` -- it is used internally by `Scanner` and not exposed in the public API. The public interface is through the module-level functions.

### Construction

Created internally by `Scanner::new()` using `Parser::new(config)`. All regex patterns are compiled once during construction.

## Public Functions

### `validate_patterns`

```rust
pub fn validate_patterns(config: &ParsingConfig) -> Result<(), TowlParserError>
```

Validates all regex patterns in the config without creating a parser. Useful for checking configuration before starting a scan.

**Checks:**

- Each pattern is valid regex
- Each pattern is within `MAX_PATTERN_LENGTH` (256 characters)
- Compiled regex is within `REGEX_SIZE_LIMIT` (256 KB)

### `parse_content`

```rust
pub fn parse_content(
    config: &ParsingConfig,
    path: &Path,
    content: &str,
) -> Result<Vec<TodoComment>, TowlParserError>
```

Parses file content for TODO comments. Creates a temporary parser, runs extraction, and returns the results.

## Parsing Pipeline

For each line in the file:

1. **Comment detection** -- Check if the line matches any `comment_prefixes` pattern
2. **TODO matching** -- Check if the comment matches any `todo_patterns` pattern
3. **Type classification** -- Determine the `TodoType` from the matched pattern
4. **Description extraction** -- Extract the description via the first capture group `(.*)`
5. **Context capture** -- Grab `include_context_lines` lines above and below
6. **Function detection** -- Search upward (within 3 lines) for a `function_patterns` match

## Pattern Types

### Comment Prefixes

Regex patterns that identify comment lines:

| Default pattern | Matches |
|-----------------|---------|
| `//` | C-style line comments |
| `^\s*#` | Shell/Python comments |
| `/\*` | C-style block comment start |
| `^\s*\*` | C-style block comment continuation |

### TODO Patterns

Regex patterns with a capture group for the description:

| Default pattern | Matches |
|-----------------|---------|
| `(?i)\bTODO:\s*(.*)` | TODO comments |
| `(?i)\bFIXME:\s*(.*)` | FIXME comments |
| `(?i)\bHACK:\s*(.*)` | HACK comments |
| `(?i)\bNOTE:\s*(.*)` | NOTE comments |
| `(?i)\bBUG:\s*(.*)` | BUG comments |

All default patterns are case-insensitive (`(?i)`).

### Function Patterns

Regex patterns to detect enclosing function names:

| Default pattern | Language |
|-----------------|----------|
| `^\s*(pub\s+)?fn\s+(\w+)` | Rust |
| `^\s*def\s+(\w+)` | Python |
| `^\s*(async\s+)?function\s+(\w+)` | JavaScript |
| `^\s*(public\|private\|protected)?\s*(static\s+)?\w+\s+(\w+)\s*\(` | Java/C# |
| `^\s*func\s+(\w+)` | Go/Swift |

## Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `MIN_CONTEXT_LINES` | 1 | Minimum context window |
| `MAX_CONTEXT_LINES` | 50 | Maximum context window |
| `FORWARD_SEARCH_LINES` | 3 | Lines searched upward for function context |
| `MAX_PATTERN_LENGTH` | 256 | Maximum regex pattern string length |
| `REGEX_SIZE_LIMIT` | 262,144 | Maximum compiled regex size (256 KB) |
| `MAX_TOTAL_PATTERNS` | 50 | Maximum total patterns across all categories |

## Errors

```rust
pub enum TowlParserError {
    InvalidRegexPattern(String, regex::Error),
    UnknownConfigPattern(TowlCommentError),
    RegexGroupMissing,
    PatternTooLong(usize, usize),
    TooManyTotalPatterns { count: usize, max_allowed: usize },
}
```

| Variant | Cause |
|---------|-------|
| `InvalidRegexPattern` | Regex failed to compile |
| `UnknownConfigPattern` | Pattern matched but type could not be determined |
| `RegexGroupMissing` | Pattern lacks a capture group |
| `PatternTooLong` | Pattern exceeds 256 characters |
| `TooManyTotalPatterns` | Total patterns across all categories exceeds 50 |
