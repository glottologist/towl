use secrecy::SecretString;
use towl::config::{GitHubConfig, Owner, ParsingConfig, Repo, TowlConfig};

#[must_use]
pub fn mock_towl_config() -> TowlConfig {
    TowlConfig {
        parsing: ParsingConfig {
            file_extensions: ["rs".to_string(), "py".to_string(), "txt".to_string()]
                .into_iter()
                .collect(),
            exclude_patterns: vec!["target/*".to_string(), "*.log".to_string()],
            comment_prefixes: vec![
                r"//".to_string(),
                r"^\s*#".to_string(),
                r"/\*".to_string(),
                r"^\s*\*".to_string(),
            ],
            todo_patterns: vec![
                r"(?i)\bTODO:\s*(.*)".to_string(),
                r"(?i)\bFIXME:\s*(.*)".to_string(),
                r"(?i)\bHACK:\s*(.*)".to_string(),
                r"(?i)\bNOTE:\s*(.*)".to_string(),
                r"(?i)\bBUG:\s*(.*)".to_string(),
            ],
            function_patterns: vec![
                r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),
                r"^\s*def\s+(\w+)".to_string(),
            ],
            include_context_lines: 3,
        },
        github: GitHubConfig::default(),
    }
}

#[must_use]
pub fn mock_github_config() -> GitHubConfig {
    GitHubConfig {
        token: SecretString::from("test-token-12345"),
        owner: Owner::new("test-owner"),
        repo: Repo::new("test-repo"),
        rate_limit_delay_ms: 0,
    }
}
