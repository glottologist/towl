use std::collections::HashSet;

use octocrab::Octocrab;
use secrecy::ExposeSecret;
use tracing::{debug, info};

use crate::comment::todo::TodoComment;
use crate::config::GitHubConfig;

use super::error::TowlGitHubError;
use super::types::CreatedIssue;

const MAX_TITLE_LENGTH: usize = 50;

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
        let mut page = 1u32;

        loop {
            let issues = self
                .client
                .issues(&self.owner, &self.repo)
                .list()
                .state(octocrab::params::State::All)
                .per_page(100)
                .page(page)
                .send()
                .await
                .map_err(|e| TowlGitHubError::from_octocrab(e, &self.owner, &self.repo))?;

            if issues.items.is_empty() {
                break;
            }

            for issue in &issues.items {
                self.existing_issue_titles.insert(issue.title.clone()); // clone: HashSet needs owned String

                if let Some(ref body) = issue.body {
                    if let Some(todo_id) = Self::extract_todo_id(body) {
                        self.existing_todo_ids.insert(todo_id);
                    }
                }
            }

            debug!("Loaded page {} ({} items)", page, issues.items.len());

            if issues.items.len() < 100 {
                break;
            }

            page += 1;
        }

        info!(
            "Loaded {} existing titles, {} TODO IDs",
            self.existing_issue_titles.len(),
            self.existing_todo_ids.len()
        );

        Ok(())
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

        let body = Self::generate_issue_body(todo);
        let label = todo.todo_type.github_label().to_string();

        self.handle_rate_limiting().await;

        let issue = self
            .client
            .issues(&self.owner, &self.repo)
            .create(&title)
            .body(&body)
            .labels(vec![label])
            .send()
            .await
            .map_err(|e| TowlGitHubError::from_octocrab(e, &self.owner, &self.repo))?;

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

    fn generate_issue_body(todo: &TodoComment) -> String {
        use std::fmt::Write;
        let mut body = String::new();

        writeln!(body, "## TODO Details\n").unwrap_or_default();
        writeln!(body, "**Type:** {}", todo.todo_type).unwrap_or_default();
        writeln!(body, "**File:** `{}`", todo.file_path.display()).unwrap_or_default();
        writeln!(body, "**Line:** {}", todo.line_number).unwrap_or_default();
        writeln!(
            body,
            "**Column:** {}-{}",
            todo.column_start, todo.column_end
        )
        .unwrap_or_default();

        writeln!(body, "\n## Description\n").unwrap_or_default();
        writeln!(body, "{}", todo.description.trim()).unwrap_or_default();

        if let Some(ref func) = todo.function_context {
            writeln!(body, "\n## Function Context\n").unwrap_or_default();
            writeln!(body, "Found in function: `{func}`").unwrap_or_default();
        }

        writeln!(body, "\n## Original Comment\n\n```").unwrap_or_default();
        writeln!(body, "{}", todo.original_text).unwrap_or_default();
        writeln!(body, "```").unwrap_or_default();

        if !todo.context_lines.is_empty() {
            writeln!(body, "\n## Context\n\n```").unwrap_or_default();
            for line in &todo.context_lines {
                writeln!(body, "{line}").unwrap_or_default();
            }
            writeln!(body, "```").unwrap_or_default();
        }

        writeln!(body, "\n---").unwrap_or_default();
        writeln!(body, "*TODO ID: {}*", todo.id).unwrap_or_default();
        write!(
            body,
            "*This issue was automatically generated by [towl](https://github.com/glottologist/towl)*"
        )
        .unwrap_or_default();

        body
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

    async fn handle_rate_limiting(&self) {
        if self.rate_limit_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.rate_limit_delay_ms)).await;
        }
    }
}

fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return "...".to_string();
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
    use crate::comment::todo::TodoType;
    use proptest::prelude::*;
    use rstest::rstest;
    use std::path::PathBuf;

    fn make_todo(desc: &str, todo_type: TodoType) -> TodoComment {
        TodoComment {
            id: "test.rs_L10_C5".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            line_number: 10,
            column_start: 5,
            column_end: 30,
            todo_type,
            original_text: format!("// {todo_type}: {desc}"),
            description: desc.to_string(),
            context_lines: vec!["9: fn main() {".to_string(), "11: }".to_string()],
            function_context: Some("main:9".to_string()),
        }
    }

    #[test]
    fn test_generate_title_short_description() {
        let todo = make_todo("Fix bug", TodoType::Todo);
        let title = GitHubClient::generate_issue_title(&todo);

        assert!(title.starts_with("[TODO]"));
        assert!(title.contains("Fix bug"));
        assert!(title.contains("(main.rs:10)"));
    }

    #[test]
    fn test_generate_body_contains_all_sections() {
        let todo = make_todo("Fix the cache", TodoType::Todo);
        let body = GitHubClient::generate_issue_body(&todo);

        assert!(body.contains("## TODO Details"));
        assert!(body.contains("**Type:** TODO"));
        assert!(body.contains("**File:** `src/main.rs`"));
        assert!(body.contains("## Function Context"));
        assert!(body.contains("*TODO ID: test.rs_L10_C5*"));
    }

    #[test]
    fn test_generate_body_without_function_context() {
        let mut todo = make_todo("Fix bug", TodoType::Bug);
        todo.function_context = None;
        let body = GitHubClient::generate_issue_body(&todo);
        assert!(!body.contains("## Function Context"));
    }

    #[rstest]
    #[case("*TODO ID: test.rs_L10_C5*", Some("test.rs_L10_C5".to_string()))]
    #[case("no id here", None)]
    #[case("*TODO ID: *", None)]
    #[case("prefix *TODO ID: my_id_123* suffix", Some("my_id_123".to_string()))]
    fn test_extract_todo_id(#[case] body: &str, #[case] expected: Option<String>) {
        assert_eq!(GitHubClient::extract_todo_id(body), expected);
    }

    #[rstest]
    #[case("short", 20, "short")]
    #[case("hello world this is long", 15, "hello world...")]
    #[case("a", 5, "a")]
    #[case("toolong", 3, "...")]
    fn test_truncate_at_word_boundary(
        #[case] input: &str,
        #[case] max_len: usize,
        #[case] expected: &str,
    ) {
        assert_eq!(truncate_at_word_boundary(input, max_len), expected);
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
            let body = GitHubClient::generate_issue_body(&todo);
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
    }
}
