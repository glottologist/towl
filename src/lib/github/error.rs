use thiserror::Error;

const DEFAULT_RATE_LIMIT_RETRY_SECS: u64 = 60;

#[derive(Error, Debug)]
pub enum TowlGitHubError {
    #[error("GitHub API error: {message}")]
    ApiError {
        message: String,
        #[source]
        source: Option<octocrab::Error>,
    },
    #[error("GitHub authentication failed: invalid or expired token")]
    AuthError,
    #[error("Rate limit exceeded, retry after {retry_after_secs}s")]
    RateLimitExceeded { retry_after_secs: u64 },
    #[error("Issue already exists: {title}")]
    IssueAlreadyExists { title: String },
    #[error("Repository not found: {owner}/{repo}")]
    RepositoryNotFound { owner: String, repo: String },
    #[error("No GitHub token configured. Set TOWL_GITHUB_TOKEN environment variable")]
    MissingToken,
}

impl TowlGitHubError {
    pub fn from_octocrab(err: octocrab::Error, owner: &str, repo: &str) -> Self {
        if let octocrab::Error::GitHub { ref source, .. } = err {
            if let Some(classified) = Self::classify_github_status(
                source.status_code.as_u16(),
                &source.message,
                owner,
                repo,
            ) {
                return classified;
            }
        }

        let msg = err.to_string(); // clone: Display → owned String for error field
        Self::ApiError {
            message: msg,
            source: Some(err),
        }
    }

    fn classify_github_status(
        status_code: u16,
        message: &str,
        owner: &str,
        repo: &str,
    ) -> Option<Self> {
        match status_code {
            401 => Some(Self::AuthError),
            404 => Some(Self::RepositoryNotFound {
                owner: owner.to_string(), // clone: error owns String
                repo: repo.to_string(),   // clone: error owns String
            }),
            403 if message.contains("rate limit") => Some(Self::RateLimitExceeded {
                retry_after_secs: DEFAULT_RATE_LIMIT_RETRY_SECS,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_classify_401_always_auth(
            message in ".*",
            owner in "[a-zA-Z0-9_-]{1,30}",
            repo in "[a-zA-Z0-9_-]{1,30}"
        ) {
            let result = TowlGitHubError::classify_github_status(401, &message, &owner, &repo);
            prop_assert!(matches!(result, Some(TowlGitHubError::AuthError)));
        }

        #[test]
        fn prop_classify_404_always_not_found(
            message in ".*",
            owner in "[a-zA-Z0-9_-]{1,30}",
            repo in "[a-zA-Z0-9_-]{1,30}"
        ) {
            let result = TowlGitHubError::classify_github_status(404, &message, &owner, &repo);
            match result {
                Some(TowlGitHubError::RepositoryNotFound { owner: o, repo: r }) => {
                    prop_assert_eq!(o, owner);
                    prop_assert_eq!(r, repo);
                }
                other => prop_assert!(false, "Expected RepositoryNotFound, got {:?}", other),
            }
        }

        #[test]
        fn prop_classify_403_with_rate_limit(
            prefix in "[a-zA-Z0-9 ]{0,50}",
            suffix in "[a-zA-Z0-9 ]{0,50}",
            owner in "[a-zA-Z0-9_-]{1,30}",
            repo in "[a-zA-Z0-9_-]{1,30}"
        ) {
            let message = format!("{prefix}rate limit{suffix}");
            let result = TowlGitHubError::classify_github_status(403, &message, &owner, &repo);
            match result {
                Some(TowlGitHubError::RateLimitExceeded { retry_after_secs }) => {
                    prop_assert_eq!(retry_after_secs, DEFAULT_RATE_LIMIT_RETRY_SECS);
                }
                other => prop_assert!(false, "Expected RateLimitExceeded, got {:?}", other),
            }
        }

        #[test]
        fn prop_classify_403_without_rate_limit(
            owner in "[a-zA-Z0-9_-]{1,30}",
            repo in "[a-zA-Z0-9_-]{1,30}"
        ) {
            let result = TowlGitHubError::classify_github_status(403, "forbidden", &owner, &repo);
            prop_assert!(result.is_none());
        }

        #[test]
        fn prop_classify_other_status_none(
            status in (0u16..=u16::MAX).prop_filter("not 401/403/404", |s| !matches!(s, 401 | 403 | 404)),
            message in ".*",
            owner in "[a-zA-Z0-9_-]{1,30}",
            repo in "[a-zA-Z0-9_-]{1,30}"
        ) {
            let result = TowlGitHubError::classify_github_status(status, &message, &owner, &repo);
            prop_assert!(result.is_none());
        }
    }
}
