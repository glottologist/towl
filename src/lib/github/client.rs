use std::collections::HashSet;

use octocrab::Octocrab;
use secrecy::ExposeSecret;
use tracing::{debug, info, warn};

use crate::comment::todo::TodoComment;
use crate::config::GitHubConfig;

use super::error::TowlGitHubError;
use super::types::CreatedIssue;

const MAX_TITLE_LENGTH: usize = 50;
const MAX_RATE_LIMIT_RETRIES: u32 = 3;
const MAX_PAGES: u32 = 100;

pub struct GitHubClient {
    client: Octocrab,
    owner: String,
    repo: String,
    existing_issue_titles: HashSet<String>,
    existing_todo_ids: HashSet<String>,
    rate_limit_delay_ms: u64,
}

impl GitHubClient {
    /// Creates a new GitHub client from config.
    ///
    /// # Errors
    /// Returns `TowlGitHubError::MissingToken` if the token is empty.
    /// Returns `TowlGitHubError::ApiError` if the client cannot be built.
    pub fn new(config: &GitHubConfig) -> Result<Self, TowlGitHubError> {
        let token = config.token.expose_secret();
        if token.is_empty() {
            return Err(TowlGitHubError::MissingToken);
        }

        let client = Octocrab::builder()
            // SECURITY: Single exposure point for GitHub token from SecretString
            .personal_token(token.to_string()) // clone: SecretString expose for API auth
            .build()
            .map_err(|e| TowlGitHubError::ApiError {
                message: e.to_string(), // clone: owned String for error variant
                source: Some(e),
            })?;

        Ok(Self {
            client,
            owner: config.owner.to_string(), // clone: owned String for struct field
            repo: config.repo.to_string(),   // clone: owned String for struct field
            existing_issue_titles: HashSet::new(),
            existing_todo_ids: HashSet::new(),
            rate_limit_delay_ms: config.rate_limit_delay_ms,
        })
    }

    /// Loads existing issues for duplicate detection.
    ///
    /// # Errors
    /// Returns `TowlGitHubError` if the API call fails.
    pub async fn load_existing_issues(&mut self) -> Result<(), TowlGitHubError> {
        for page in 1..=MAX_PAGES {
            let has_more = self.load_issues_page(page).await?;
            if !has_more {
                break;
            }
        }

        info!(
            "Loaded {} existing titles, {} TODO IDs",
            self.existing_issue_titles.len(),
            self.existing_todo_ids.len()
        );

        Ok(())
    }

    async fn load_issues_page(&mut self, page: u32) -> Result<bool, TowlGitHubError> {
        let issues = self.fetch_issues_page(page).await?;

        if issues.items.is_empty() {
            return Ok(false);
        }

        let count = issues.items.len();
        for issue in &issues.items {
            self.record_existing_issue(issue);
        }

        debug!("Loaded page {page} ({count} items)");

        if count < 100 || page >= MAX_PAGES {
            if page >= MAX_PAGES {
                warn!("Pagination limit reached ({MAX_PAGES} pages), stopping issue loading");
            }
            return Ok(false);
        }

        Ok(true)
    }

    async fn fetch_issues_page(
        &self,
        page: u32,
    ) -> Result<octocrab::Page<octocrab::models::issues::Issue>, TowlGitHubError> {
        self.client
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(100)
            .page(page)
            .send()
            .await
            .map_err(|e| TowlGitHubError::from_octocrab(e, &self.owner, &self.repo))
    }

    fn record_existing_issue(&mut self, issue: &octocrab::models::issues::Issue) {
        self.existing_issue_titles.insert(issue.title.clone()); // clone: HashSet needs owned String
        if let Some(ref body) = issue.body {
            if let Some(todo_id) = Self::extract_todo_id(body) {
                self.existing_todo_ids.insert(todo_id);
            }
        }
    }

    #[must_use]
    pub fn issue_exists(&self, todo: &TodoComment) -> bool {
        if self.existing_todo_ids.contains(&todo.id) {
            return true;
        }
        let title = Self::generate_issue_title(todo);
        self.existing_issue_titles.contains(&title)
    }

    /// Creates a GitHub issue for a TODO comment.
    ///
    /// # Errors
    /// Returns `TowlGitHubError::IssueAlreadyExists` for duplicates.
    /// Returns other `TowlGitHubError` variants for API failures.
    pub async fn create_issue(
        &mut self,
        todo: &TodoComment,
    ) -> Result<CreatedIssue, TowlGitHubError> {
        let title = Self::generate_issue_title(todo);

        if self.issue_exists(todo) {
            return Err(TowlGitHubError::IssueAlreadyExists { title });
        }

        let body = Self::generate_issue_body(todo).map_err(|e| TowlGitHubError::ApiError {
            message: format!("Failed to format issue body: {e}"),
            source: None,
        })?;
        let label = todo.todo_type.github_label();

        let issue = self.create_issue_with_retry(&title, &body, label).await?;

        let html_url = issue.html_url.to_string(); // clone: Url → owned String for CreatedIssue

        self.existing_issue_titles.insert(title.clone()); // clone: insert needs owned, title reused below
        self.existing_todo_ids.insert(todo.id.clone()); // clone: insert needs owned String

        info!("Created issue #{}: {}", issue.number, title);

        Ok(CreatedIssue::new(
            issue.number,
            title,
            html_url,
            todo.id.clone(), // clone: CreatedIssue needs owned String
        ))
    }

    fn generate_issue_title(todo: &TodoComment) -> String {
        let type_prefix = format!("[{}]", todo.todo_type);
        let file_name = todo
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let location_suffix = format!("({}:{})", file_name, todo.line_number);

        let overhead = type_prefix.len() + location_suffix.len() + 2;
        let available = MAX_TITLE_LENGTH.saturating_sub(overhead);

        let desc = todo.description.trim();
        let truncated_desc = truncate_at_word_boundary(desc, available);

        format!("{type_prefix} {truncated_desc} {location_suffix}")
    }

    fn generate_issue_body(todo: &TodoComment) -> Result<String, std::fmt::Error> {
        use std::fmt::Write;
        let mut body = String::new();
        let file_display = todo.file_path.display().to_string(); // clone: owned String for format! interpolation

        write!(
            body,
            "## TODO Details\n\n\
             **Type:** {}\n\
             **File:** {}\n\
             **Line:** {}\n\
             **Column:** {}-{}\n\n\
             ## Description\n\n\
             {}\n",
            todo.todo_type,
            sanitize_for_inline_code(&file_display),
            todo.line_number,
            todo.column_start,
            todo.column_end,
            escape_markdown(todo.description.trim()),
        )?;

        if let Some(ref func) = todo.function_context {
            write!(
                body,
                "\n## Function Context\n\nFound in function: {}\n",
                sanitize_for_inline_code(func),
            )?;
        }

        write!(
            body,
            "\n## Original Comment\n\n{}\n",
            code_block(&todo.original_text),
        )?;

        if !todo.context_lines.is_empty() {
            let context = todo.context_lines.join("\n");
            write!(body, "\n## Context\n\n{}\n", code_block(&context))?;
        }

        write!(
            body,
            "\n---\n\
             *TODO ID: {}*\n\
             *This issue was automatically generated by \
             [towl](https://github.com/glottologist/towl)*",
            todo.id,
        )?;

        Ok(body)
    }

    fn extract_todo_id(body: &str) -> Option<String> {
        let prefix = "*TODO ID: ";
        let start = body.find(prefix)?;
        let id_start = start + prefix.len();
        let remaining = &body[id_start..];
        let end = remaining.find('*')?;

        let id = remaining[..end].trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string()) // clone: owned String from borrowed slice
        }
    }

    async fn create_issue_with_retry(
        &self,
        title: &str,
        body: &str,
        label: &str,
    ) -> Result<octocrab::models::issues::Issue, TowlGitHubError> {
        let mut attempts = 0u32;
        loop {
            self.handle_rate_limiting().await;
            match self
                .client
                .issues(&self.owner, &self.repo)
                .create(title)
                .body(body)
                .labels(vec![label.to_string()]) // clone: API requires owned String
                .send()
                .await
            {
                Ok(issue) => return Ok(issue),
                Err(e) => {
                    let err = TowlGitHubError::from_octocrab(e, &self.owner, &self.repo);
                    if let TowlGitHubError::RateLimitExceeded { retry_after_secs } = &err {
                        attempts = attempts.saturating_add(1);
                        if attempts < MAX_RATE_LIMIT_RETRIES {
                            warn!(
                                "Rate limited, retrying after {retry_after_secs}s \
                                 (attempt {attempts}/{MAX_RATE_LIMIT_RETRIES})"
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(*retry_after_secs))
                                .await;
                            continue;
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    async fn handle_rate_limiting(&self) {
        if self.rate_limit_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.rate_limit_delay_ms)).await;
        }
    }
}

fn max_backtick_run(s: &str) -> usize {
    s.bytes()
        .fold((0_usize, 0_usize), |(max, cur), b| {
            if b == b'`' {
                let next = cur.saturating_add(1);
                (max.max(next), next)
            } else {
                (max, 0)
            }
        })
        .0
}

fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(s.len() / 4));
    for ch in s.chars() {
        if matches!(
            ch,
            '\\' | '`' | '*' | '_' | '[' | ']' | '#' | '!' | '<' | '>' | '~' | '|'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn sanitize_for_inline_code(s: &str) -> String {
    let max_run = max_backtick_run(s);
    if max_run == 0 {
        return format!("`{s}`");
    }
    let fence_len = max_run.saturating_add(1);
    let fence: String = "`".repeat(fence_len);
    format!("{fence} {s} {fence}")
}

fn code_block(content: &str) -> String {
    let fence_len = max_backtick_run(content).saturating_add(1).max(3);
    let fence: String = "`".repeat(fence_len);
    format!("{fence}\n{content}\n{fence}")
}

fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string(); // clone: &str → owned String for return
    }
    if max_len <= 3 {
        return "...".to_string(); // clone: &str → owned String for return
    }

    let target = max_len - 3;
    let boundary = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= target)
        .last()
        .unwrap_or(0);

    let truncated = &s[..boundary];
    if let Some(last_space) = truncated.rfind(' ') {
        if last_space > 0 {
            return format!("{}...", &truncated[..last_space]);
        }
    }

    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::test_support::TestTodoBuilder;
    use crate::comment::todo::TodoType;
    use proptest::prelude::*;
    use rstest::rstest;

    fn make_todo(desc: &str, todo_type: TodoType) -> TodoComment {
        TestTodoBuilder::new()
            .description(desc)
            .todo_type(todo_type)
            .file_path("src/main.rs")
            .line_number(10)
            .column_start(5)
            .column_end(30)
            .context_lines(vec!["9: fn main() {".to_string(), "11: }".to_string()])
            .function_context("main:9")
            .build()
    }

    #[test]
    fn test_generate_body_contains_all_sections() {
        let todo = make_todo("Fix the cache", TodoType::Todo);
        let body = GitHubClient::generate_issue_body(&todo).unwrap();

        assert!(body.contains("## TODO Details"));
        assert!(body.contains("**Type:** TODO"));
        assert!(body.contains("**File:** `src/main.rs`"), "body: {body}");
        assert!(body.contains("## Function Context"));
        assert!(body.contains("*TODO ID: src/main.rs_L10*"));
    }

    #[test]
    fn test_generate_body_without_function_context() {
        let mut todo = make_todo("Fix bug", TodoType::Bug);
        todo.function_context = None;
        let body = GitHubClient::generate_issue_body(&todo).unwrap();
        assert!(!body.contains("## Function Context"));
    }

    #[test]
    fn test_generate_body_without_context_lines() {
        let mut todo = make_todo("Fix bug", TodoType::Bug);
        todo.context_lines = vec![];
        let body = GitHubClient::generate_issue_body(&todo).unwrap();
        assert!(!body.contains("## Context"));
        assert!(body.contains("## TODO Details"));
        assert!(body.contains("*TODO ID:"));
    }

    #[rstest]
    #[case("*TODO ID: test.rs_L10_C5*", Some("test.rs_L10_C5".to_string()))]
    #[case("no id here", None)]
    #[case("*TODO ID: *", None)]
    #[case("prefix *TODO ID: my_id_123* suffix", Some("my_id_123".to_string()))]
    fn test_extract_todo_id(#[case] body: &str, #[case] expected: Option<String>) {
        assert_eq!(GitHubClient::extract_todo_id(body), expected);
    }

    proptest! {
        #[test]
        fn prop_title_bounded(
            desc in "[a-zA-Z0-9 ]{1,200}",
            todo_type in prop::sample::select(vec![
                TodoType::Todo, TodoType::Fixme, TodoType::Hack,
                TodoType::Note, TodoType::Bug
            ])
        ) {
            let todo = make_todo(&desc, todo_type);
            let title = GitHubClient::generate_issue_title(&todo);
            prop_assert!(title.len() < 200);
            prop_assert!(title.starts_with('['));
        }

        #[test]
        fn prop_body_contains_todo_id(
            desc in "[a-zA-Z0-9 ]{1,100}",
            id in "[a-zA-Z0-9_.]{1,50}"
        ) {
            let mut todo = make_todo(&desc, TodoType::Todo);
            todo.id = id.clone(); // clone: proptest needs owned for assertion
            let body = GitHubClient::generate_issue_body(&todo).unwrap();
            let expected = ["*TODO ID: ", &id, "*"].join("");
            prop_assert!(body.contains(&expected));
        }

        #[test]
        fn prop_extract_todo_id_roundtrip(
            id in "[a-zA-Z0-9_.-]{1,50}"
        ) {
            let body = format!("text\n*TODO ID: {id}*\nmore");
            let extracted = GitHubClient::extract_todo_id(&body);
            prop_assert_eq!(extracted, Some(id));
        }

        #[test]
        fn prop_truncate_never_panics(
            s in "\\PC{0,200}",
            max_len in 0usize..100
        ) {
            let _ = truncate_at_word_boundary(&s, max_len);
        }

        #[test]
        fn prop_truncate_respects_max_len(
            s in "\\PC{0,200}",
            max_len in 3usize..100
        ) {
            let result = truncate_at_word_boundary(&s, max_len);
            prop_assert!(
                result.len() <= max_len,
                "truncate({:?}, {}) produced {:?} (len {})",
                s, max_len, result, result.len()
            );
        }

        #[test]
        fn prop_truncate_short_input_unchanged(
            s in "[a-zA-Z0-9 ]{1,20}",
            extra in 0usize..30
        ) {
            let max_len = s.len() + extra;
            let result = truncate_at_word_boundary(&s, max_len);
            prop_assert_eq!(result, s, "Input shorter than max_len should be unchanged");
        }

        #[test]
        fn prop_truncate_ends_with_ellipsis_when_truncated(
            s in "[a-zA-Z0-9 ]{10,200}",
            max_len in 4usize..9
        ) {
            let result = truncate_at_word_boundary(&s, max_len);
            prop_assert!(
                result.ends_with("..."),
                "Truncated result {:?} should end with '...'", result
            );
        }

        #[test]
        fn prop_truncate_max_len_lte_3_returns_ellipsis(
            s in "[a-zA-Z0-9]{4,50}",
            max_len in 0usize..=3
        ) {
            let result = truncate_at_word_boundary(&s, max_len);
            prop_assert_eq!(result, "...");
        }

        #[test]
        fn prop_code_block_contains_content(content in "\\PC{0,200}") {
            let result = code_block(&content);
            prop_assert!(result.contains(&content));
            let lines: Vec<&str> = result.lines().collect();
            prop_assert!(lines.len() >= 3);
            prop_assert!(lines[0].chars().all(|c| c == '`'));
            prop_assert!(lines.last().unwrap().chars().all(|c| c == '`'));
        }

        #[test]
        fn prop_sanitize_inline_wraps_content(content in "[a-zA-Z0-9 ]{1,100}") {
            let result = sanitize_for_inline_code(&content);
            prop_assert!(result.contains(&content));
            prop_assert!(result.starts_with('`'));
            prop_assert!(result.ends_with('`'));
        }

        #[test]
        fn prop_escape_preserves_alphanumeric(s in "[a-zA-Z0-9 ]{1,100}") {
            let result = escape_markdown(&s);
            prop_assert_eq!(result, s);
        }
    }

    #[rstest]
    #[case("hello world", "hello world")]
    #[case("*bold*", "\\*bold\\*")]
    #[case("`code`", "\\`code\\`")]
    #[case("# heading", "\\# heading")]
    #[case("<html>", "\\<html\\>")]
    #[case("a | b", "a \\| b")]
    #[case("~~strike~~", "\\~\\~strike\\~\\~")]
    fn test_escape_markdown_metacharacters(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(escape_markdown(input), expected);
    }

    #[rstest]
    #[case("no backticks", "`no backticks`")]
    #[case("has ` one", "`` has ` one ``")]
    #[case("has `` two", "``` has `` two ```")]
    fn test_sanitize_inline_code_backticks(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(sanitize_for_inline_code(input), expected);
    }

    #[rstest]
    #[case("normal text", "```\nnormal text\n```")]
    #[case("has ``` triple", "````\nhas ``` triple\n````")]
    fn test_code_block_fence_length(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(code_block(input), expected);
    }
}
