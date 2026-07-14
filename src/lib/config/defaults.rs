use std::collections::HashSet;

const RUST_COMMENT_PREFIX: &str = r"//";
const SHELL_COMMENT_PREFIX: &str = r"^\s*#";
const C_MULTILINE_START: &str = r"/\*";
const MULTILINE_CONTINUATION: &str = r"^\s*\*";

pub(super) fn default_file_extensions() -> HashSet<String> {
    [
        "rs".to_string(),
        "toml".to_string(),
        "json".to_string(),
        "yaml".to_string(),
        "yml".to_string(),
        "sh".to_string(),
        "bash".to_string(),
    ]
    .into_iter()
    .collect()
}

pub(super) fn default_exclude_patterns() -> Vec<String> {
    vec!["target/*".to_string(), ".git/*".to_string()]
}

pub(super) const fn default_include_context_lines() -> usize {
    10
}

pub(super) fn default_comment_prefixes() -> Vec<String> {
    vec![
        RUST_COMMENT_PREFIX.to_string(),
        SHELL_COMMENT_PREFIX.to_string(),
        C_MULTILINE_START.to_string(),
        MULTILINE_CONTINUATION.to_string(),
    ]
}

pub(super) fn default_todo_patterns() -> Vec<String> {
    vec![
        r"(?i)\bTODO:\s*(.*)".to_string(),
        r"(?i)\bFIXME:\s*(.*)".to_string(),
        r"(?i)\bHACK:\s*(.*)".to_string(),
        r"(?i)\bNOTE:\s*(.*)".to_string(),
        r"(?i)\bBUG:\s*(.*)".to_string(),
    ]
}

pub(super) fn default_function_patterns() -> Vec<String> {
    vec![
        r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),
        r"^\s*def\s+(\w+)".to_string(),
        r"^\s*(async\s+)?function\s+(\w+)".to_string(),
        r"^\s*(?:public|private|protected)\s+(?:static\s+)?\w+\s+(\w+)\s*\(".to_string(),
        r"^\s*func\s+(\w+)".to_string(),
    ]
}

pub(super) const fn default_rate_limit_delay_ms() -> u64 {
    1000
}

pub(super) fn default_llm_provider() -> String {
    "claude".to_string()
}

pub fn default_llm_model() -> String {
    "claude-opus-4-8".to_string()
}

pub(super) const fn default_max_concurrent_analyses() -> usize {
    5
}

pub(super) const fn default_max_analyse_count() -> usize {
    50
}

pub(super) const fn default_llm_max_retries() -> usize {
    3
}

pub(super) const fn default_llm_max_tokens() -> u32 {
    4096
}
