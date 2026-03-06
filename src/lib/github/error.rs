use thiserror::Error;

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
        let message = err.to_string();

        if message.contains("401") || message.contains("Unauthorized") {
            return Self::AuthError;
        }
        if message.contains("404") || message.contains("Not Found") {
            return Self::RepositoryNotFound {
                owner: owner.to_string(),
                repo: repo.to_string(),
            };
        }
        if message.contains("403") && message.contains("rate limit") {
            return Self::RateLimitExceeded {
                retry_after_secs: 60,
            };
        }

        Self::ApiError {
            message,
            source: Some(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        let err = TowlGitHubError::AuthError;
        assert!(err.to_string().contains("authentication"));

        let err = TowlGitHubError::MissingToken;
        assert!(err.to_string().contains("TOWL_GITHUB_TOKEN"));

        let err = TowlGitHubError::RepositoryNotFound {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
        };
        assert!(err.to_string().contains("owner/repo"));
    }
}
