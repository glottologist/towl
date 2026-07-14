use std::collections::HashSet;
use std::path::Path;

use octocrab::Octocrab;
use secrecy::ExposeSecret;
use tracing::{debug, info, warn};
use url::Url;

use crate::comment::todo::TodoComment;
use crate::config::GitHubConfig;
use crate::{escape_markdown, max_backtick_run, sanitize_for_inline_code};

use super::error::TowlGitHubError;
use super::types::CreatedIssue;

const MAX_TITLE_LENGTH: usize = 256;
const MAX_RATE_LIMIT_RETRIES: u32 = 3;
const MAX_PAGES: u32 = 100;

/// Authenticated GitHub API client for creating issues from TODO comments.
///
/// Maintains a cache of existing issue titles and TODO IDs for deduplication.
/// Includes rate-limit handling with configurable delays and automatic retries.
pub struct GitHubClient {
    client: Octocrab,
    owner: String,
    repo: String,
    default_branch: String,
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
            .personal_token(token.to_string())
            .build()
            .map_err(|e| TowlGitHubError::ApiError {
                message: e.to_string(),
                source: Some(e),
            })?;

        Ok(Self {
            client,
            owner: config.owner.to_string(),
            repo: config.repo.to_string(),
            default_branch: "main".to_string(),
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
        self.fetch_default_branch().await;

        for page in 1..=MAX_PAGES {
            let has_more = self.load_issues_page(page).await?;
            if !has_more {
                break;
            }
        }

        info!(
            "Loaded {} existing titles, {} TODO IDs (default branch: {})",
            self.existing_issue_titles.len(),
            self.existing_todo_ids.len(),
            self.default_branch,
        );

        Ok(())
    }

    async fn fetch_default_branch(&mut self) {
        match self.client.repos(&self.owner, &self.repo).get().await {
            Ok(repo) => {
                if let Some(branch) = repo.default_branch {
                    debug!("Detected default branch: {branch}");
                    self.default_branch = branch;
                }
            }
            Err(e) => {
                warn!(
                    "Could not fetch default branch, using '{}': {e}",
                    self.default_branch
                );
            }
        }
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
        // the /issues endpoint also returns pull requests; a PR title must not
        // suppress issue creation
        if issue.pull_request.is_some() {
            return;
        }
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

        let body = Self::generate_issue_body(todo, &self.owner, &self.repo, &self.default_branch)
            .map_err(|e| TowlGitHubError::ApiError {
            message: format!("Failed to format issue body: {e}"),
            source: None,
        })?;
        let label = todo.todo_type.github_label();

        let issue = self.create_issue_with_retry(&title, &body, label).await?;

        let html_url = issue.html_url.to_string();

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
        let type_prefix = todo.todo_type.as_filter_str();
        let desc = todo.description.trim();

        if desc.len() + type_prefix.len() + 2 <= MAX_TITLE_LENGTH {
            format!("{type_prefix}: {desc}")
        } else {
            let available = MAX_TITLE_LENGTH.saturating_sub(type_prefix.len() + 2);
            let truncated = truncate_at_word_boundary(desc, available);
            format!("{type_prefix}: {truncated}")
        }
    }

    fn generate_issue_body(
        todo: &TodoComment,
        owner: &str,
        repo: &str,
        default_branch: &str,
    ) -> Result<String, std::fmt::Error> {
        use std::fmt::Write;
        let mut body = String::new();
        let file_display = todo.file_path.display().to_string();

        let location_line = build_file_url(
            owner,
            repo,
            default_branch,
            &todo.file_path,
            todo.line_number,
        )
        .map_or_else(
            || {
                format!(
                    "**File:** {file}\n**Line:** {line}\n**Column:** {col_start}-{col_end}",
                    file = sanitize_for_inline_code(&file_display),
                    line = todo.line_number,
                    col_start = todo.column_start,
                    col_end = todo.column_end,
                )
            },
            |url| {
                format!(
                    "**Location:** [`{file_display}:{line}`]({url}) (columns {col_start}-{col_end})",
                    line = todo.line_number,
                    col_start = todo.column_start,
                    col_end = todo.column_end,
                )
            },
        );

        write!(
            body,
            "## TODO Details\n\n\
             **Type:** {}\n\
             {location_line}\n\n\
             ## Description\n\n\
             {}\n",
            todo.todo_type,
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

        if let Some(ref analysis) = todo.analysis {
            write!(
                body,
                "\n## AI Analysis\n\n\
                 **Validity:** {}\n\
                 **Confidence:** {:.0}%\n\n\
                 ### Reasoning\n\n{}\n\n\
                 ### Enhanced Description\n\n{}\n",
                analysis.validity,
                analysis.confidence * 100.0,
                escape_markdown(&analysis.reasoning),
                escape_markdown(&analysis.enrichment),
            )?;
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
            Some(id.to_string())
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
                .labels(vec![label.to_string()])
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

fn build_file_url(
    owner: &str,
    repo: &str,
    default_branch: &str,
    file_path: &Path,
    line_number: usize,
) -> Option<Url> {
    let mut url = Url::parse("https://github.com").ok()?;
    {
        let mut segments = url.path_segments_mut().ok()?;
        segments.push(owner);
        segments.push(repo);
        segments.push("blob");
        segments.push(default_branch);
        // Path::components is separator-agnostic, so Windows paths do not end
        // up as one percent-encoded backslash segment; "." and ".." drop out
        for component in file_path.components() {
            if let std::path::Component::Normal(segment) = component {
                segments.push(&segment.to_string_lossy());
            }
        }
    }
    let fragment = format!("L{line_number}");
    url.set_fragment(Some(&fragment));
    Some(url)
}

fn code_block(content: &str) -> String {
    let fence_len = max_backtick_run(content).saturating_add(1).max(3);
    let fence: String = "`".repeat(fence_len);
    format!("{fence}\n{content}\n{fence}")
}

fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return "...".to_string();
    }

    let target = max_len.saturating_sub(3);
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

    const TEST_OWNER: &str = "testowner";
    const TEST_REPO: &str = "testrepo";
    const TEST_BRANCH: &str = "main";

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

    fn body_for(todo: &TodoComment) -> String {
        GitHubClient::generate_issue_body(todo, TEST_OWNER, TEST_REPO, TEST_BRANCH).unwrap()
    }

    #[test]
    fn test_generate_body_contains_all_sections() {
        let todo = make_todo("Fix the cache", TodoType::Todo);
        let body = body_for(&todo);

        assert!(body.contains("## TODO Details"));
        assert!(body.contains("**Type:** TODO"));
        assert!(
            body.contains("**Location:** [`src/main.rs:10`](https://github.com/testowner/testrepo/blob/main/src/main.rs#L10)"),
            "body: {body}"
        );
        assert!(body.contains("## Function Context"));
        assert!(body.contains("*TODO ID: src/main.rs_L10*"));
    }

    #[rstest]
    #[case("function_context", "## Function Context")]
    #[case("context_lines", "## Context")]
    #[case("analysis", "AI Analysis")]
    fn test_generate_body_section_absent(#[case] field: &str, #[case] marker: &str) {
        let mut todo = make_todo("Fix bug", TodoType::Bug);
        match field {
            "function_context" => todo.function_context = None,
            "context_lines" => todo.context_lines = vec![],
            "analysis" => {} // analysis is already None by default
            _ => return,
        }
        let body = body_for(&todo);
        assert!(!body.contains(marker));
    }

    #[test]
    fn test_generate_body_includes_ai_analysis() {
        use crate::llm::types::{AnalysisResult, Validity};

        let mut todo = make_todo("Fix the cache", TodoType::Todo);
        todo.analysis = Some(AnalysisResult {
            validity: Validity::Valid,
            reasoning: "Cache implementation is missing".to_string(),
            is_resolved: false,
            is_relevant: true,
            is_actionable: true,
            confidence: 0.9,
            enrichment: "The caching layer needs to be added".to_string(),
        });
        let body = body_for(&todo);
        assert!(body.contains("## AI Analysis"));
        assert!(body.contains("Cache implementation is missing"));
        assert!(body.contains("Valid"));
        assert!(body.contains("90%"));
        assert!(body.contains("caching layer"));
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
        fn prop_title_uses_conventional_format(
            desc in "[a-zA-Z0-9 ]{1,200}",
            todo_type in prop::sample::select(vec![
                TodoType::Todo, TodoType::Fixme, TodoType::Hack,
                TodoType::Note, TodoType::Bug
            ])
        ) {
            let todo = make_todo(&desc, todo_type);
            let title = GitHubClient::generate_issue_title(&todo);
            let prefix = todo_type.as_filter_str();
            prop_assert!(
                title.starts_with(&format!("{prefix}:")),
                "Title {:?} should start with {:?}:", title, prefix
            );
            prop_assert!(title.len() <= MAX_TITLE_LENGTH);
            prop_assert!(!title.contains('['));
            prop_assert!(!title.contains(']'));
        }

        #[test]
        fn prop_body_contains_todo_id(
            desc in "[a-zA-Z0-9 ]{1,100}",
            id in "[a-zA-Z0-9_.]{1,50}"
        ) {
            let mut todo = make_todo(&desc, TodoType::Todo);
            todo.id = id.clone(); // clone: proptest needs owned for assertion
            let body = body_for(&todo);
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

    #[test]
    fn test_build_file_url_basic() {
        let url = build_file_url("owner", "repo", "main", Path::new("src/lib.rs"), 42).unwrap();
        assert_eq!(
            url.as_str(),
            "https://github.com/owner/repo/blob/main/src/lib.rs#L42"
        );
    }

    #[test]
    fn test_build_file_url_strips_dot_slash() {
        let url = build_file_url("owner", "repo", "main", Path::new("./src/lib.rs"), 1).unwrap();
        assert_eq!(
            url.as_str(),
            "https://github.com/owner/repo/blob/main/src/lib.rs#L1"
        );
    }

    #[test]
    fn test_build_file_url_nested_path() {
        let url = build_file_url(
            "org",
            "project",
            "develop",
            Path::new("crates/core/src/parser.rs"),
            100,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://github.com/org/project/blob/develop/crates/core/src/parser.rs#L100"
        );
    }

    proptest! {
        #[test]
        fn prop_build_file_url_always_produces_valid_url(
            owner in "[a-zA-Z0-9_-]{1,30}",
            repo in "[a-zA-Z0-9_-]{1,30}",
            branch in "[a-zA-Z0-9_.-]{1,30}",
            path_segments in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5),
            line in 1usize..10000
        ) {
            let file_path = path_segments.join("/");
            let url = build_file_url(&owner, &repo, &branch, Path::new(&file_path), line);
            prop_assert!(url.is_some());
            let url = url.unwrap();
            prop_assert!(url.as_str().starts_with("https://github.com/"));
            prop_assert!(url.fragment().is_some());
            let fragment = url.fragment().unwrap();
            prop_assert!(fragment.starts_with('L'));
        }

        #[test]
        fn prop_build_file_url_contains_all_components(
            owner in "[a-zA-Z0-9]{1,20}",
            repo in "[a-zA-Z0-9]{1,20}",
            branch in "[a-zA-Z0-9]{1,20}",
            filename in "[a-zA-Z0-9]{1,20}\\.rs",
            line in 1usize..10000
        ) {
            let url = build_file_url(&owner, &repo, &branch, Path::new(&filename), line).unwrap();
            let url_str = url.as_str();
            let expected_fragment = format!("#L{line}");
            prop_assert!(url_str.contains(&owner));
            prop_assert!(url_str.contains(&repo));
            prop_assert!(url_str.contains(&branch));
            prop_assert!(url_str.contains(&filename));
            prop_assert!(url_str.contains(&expected_fragment));
        }

        #[test]
        fn prop_body_location_contains_clickable_link(
            desc in "[a-zA-Z0-9 ]{1,50}",
            todo_type in prop::sample::select(vec![
                TodoType::Todo, TodoType::Fixme, TodoType::Hack,
                TodoType::Note, TodoType::Bug
            ])
        ) {
            let todo = make_todo(&desc, todo_type);
            let body = body_for(&todo);
            prop_assert!(
                body.contains("**Location:**"),
                "Body should contain Location field: {body}"
            );
            prop_assert!(
                body.contains("https://github.com/testowner/testrepo/blob/main/src/main.rs#L10"),
                "Body should contain GitHub blob URL: {body}"
            );
        }
    }
}
