use std::collections::HashSet;

const RUST_COMMENT_PREFIX: &str = r"//";
const SHELL_COMMENT_PREFIX: &str = r"^\s*#";
const C_MULTILINE_START: &str = r"/\*";
const MULTILINE_CONTINUATION: &str = r"^\s*\*";

pub(super) fn default_file_extensions() -> HashSet<String> {
    [
        "rs".to_string(),   // clone: &str → owned String for HashSet
        "toml".to_string(), // clone: &str → owned String for HashSet
        "json".to_string(), // clone: &str → owned String for HashSet
        "yaml".to_string(), // clone: &str → owned String for HashSet
        "yml".to_string(),  // clone: &str → owned String for HashSet
        "sh".to_string(),   // clone: &str → owned String for HashSet
        "bash".to_string(), // clone: &str → owned String for HashSet
    ]
    .into_iter()
    .collect()
}

pub(super) fn default_exclude_patterns() -> Vec<String> {
    vec![
        "target/*".to_string(), // clone: &str → owned String for Vec
        ".git/*".to_string(),   // clone: &str → owned String for Vec
    ]
}

pub(super) const fn default_include_context_lines() -> usize {
    10
}

pub(super) fn default_comment_prefixes() -> Vec<String> {
    vec![
        RUST_COMMENT_PREFIX.to_string(), // clone: &str → owned String for Vec
        SHELL_COMMENT_PREFIX.to_string(), // clone: &str → owned String for Vec
        C_MULTILINE_START.to_string(),   // clone: &str → owned String for Vec
        MULTILINE_CONTINUATION.to_string(), // clone: &str → owned String for Vec
    ]
}

pub(super) fn default_todo_patterns() -> Vec<String> {
    vec![
        r"(?i)\bTODO:\s*(.*)".to_string(), // clone: &str → owned String for Vec
        r"(?i)\bFIXME:\s*(.*)".to_string(), // clone: &str → owned String for Vec
        r"(?i)\bHACK:\s*(.*)".to_string(), // clone: &str → owned String for Vec
        r"(?i)\bNOTE:\s*(.*)".to_string(), // clone: &str → owned String for Vec
        r"(?i)\bBUG:\s*(.*)".to_string(),  // clone: &str → owned String for Vec
    ]
}

pub(super) fn default_function_patterns() -> Vec<String> {
    vec![
        r"^\s*(pub\s+)?fn\s+(\w+)".to_string(), // clone: &str → owned String for Vec
        r"^\s*def\s+(\w+)".to_string(),         // clone: &str → owned String for Vec
        r"^\s*(async\s+)?function\s+(\w+)".to_string(), // clone: &str → owned String for Vec
        r"^\s*(?:public|private|protected)\s+(?:static\s+)?\w+\s+(\w+)\s*\(".to_string(), // clone: &str → owned String for Vec
        r"^\s*func\s+(\w+)".to_string(), // clone: &str → owned String for Vec
    ]
}

pub(super) const fn default_rate_limit_delay_ms() -> u64 {
    1000
}
