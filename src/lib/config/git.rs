use super::{error::TowlConfigError, Owner, Repo};
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepoInfo {
    pub owner: Owner,
    pub repo: Repo,
}

impl GitRepoInfo {
    /// Discovers a git repository at the given path and extracts GitHub owner/repo.
    ///
    /// # Errors
    /// Returns `TowlConfigError::GitRepoNotFound` if no git repo is found,
    /// `TowlConfigError::GitRemoteNotFound` if no origin remote exists,
    /// or `TowlConfigError::GitInvalidUrl` if the URL is not a valid GitHub URL.
    pub(crate) async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, TowlConfigError> {
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(path.as_ref())
            .output()
            .await
            .map_err(|e| TowlConfigError::GitRepoNotFound {
                message: format!("Failed to run git command: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not a git repository") {
                return Err(TowlConfigError::GitRepoNotFound {
                    message: "Not a git repository".to_string(),
                });
            }
            return Err(TowlConfigError::GitRemoteNotFound {
                message: format!("Failed to find 'origin' remote: {}", stderr.trim()),
            });
        }

        let url = String::from_utf8_lossy(&output.stdout);
        Self::parse_github_url(url.trim())
    }

    fn parse_github_url(url: &str) -> Result<Self, TowlConfigError> {
        let url = url.trim();

        let path = url
            .strip_prefix("git@github.com:")
            .or_else(|| url.strip_prefix("https://github.com/"))
            .ok_or_else(|| TowlConfigError::GitInvalidUrl {
                url: url.to_string(),
                message: "URL is not a GitHub repository".to_string(),
            })?
            .trim_end_matches(".git");

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 2 {
            return Err(TowlConfigError::GitInvalidUrl {
                url: url.to_string(),
                message: "Invalid URL format: expected owner/repo".to_string(),
            });
        }

        Ok(Self {
            owner: Owner::new(parts[0]),
            repo: Repo::new(parts[1]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn valid_repo_name()(name in "[a-zA-Z0-9_-]{1,50}") -> String {
            name
        }
    }

    prop_compose! {
        fn valid_owner_name()(name in "[a-zA-Z0-9_-]{1,39}") -> String {
            name
        }
    }

    proptest! {
        #[test]
        fn prop_test_ssh_url_parsing(
            owner in valid_owner_name(),
            repo in valid_repo_name()
        ) {
            let url = format!("git@github.com:{owner}/{repo}.git");
            let result = GitRepoInfo::parse_github_url(&url);

            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert_eq!(info.owner, Owner::new(owner));
            prop_assert_eq!(info.repo, Repo::new(repo));
        }

        #[test]
        fn prop_test_https_url_parsing(
            owner in valid_owner_name(),
            repo in valid_repo_name()
        ) {
            let url = format!("https://github.com/{owner}/{repo}.git");
            let result = GitRepoInfo::parse_github_url(&url);

            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert_eq!(info.owner, Owner::new(owner));
            prop_assert_eq!(info.repo, Repo::new(repo));
        }

        #[test]
        fn prop_test_invalid_hosts_always_fail(
            host in "[a-z]{3,20}\\.(com|org|net)",
            owner in valid_owner_name(),
            repo in valid_repo_name()
        ) {
            prop_assume!(host != "github.com");

            let ssh_url = format!("git@{host}:{owner}/{repo}.git");
            let https_url = format!("https://{host}/{owner}/{repo}.git");

            prop_assert!(GitRepoInfo::parse_github_url(&ssh_url).is_err());
            prop_assert!(GitRepoInfo::parse_github_url(&https_url).is_err());
        }

        #[test]
        fn prop_test_malformed_paths_fail(
            parts in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 0..2),
        ) {
            prop_assume!(parts.len() != 2); // We want to test invalid cases

            let path = parts.join("/");
            let ssh_url = format!("git@github.com:{path}");
            let https_url = format!("https://github.com/{path}");

            prop_assert!(GitRepoInfo::parse_github_url(&ssh_url).is_err());
            prop_assert!(GitRepoInfo::parse_github_url(&https_url).is_err());
        }

        #[test]
        fn prop_test_whitespace_normalization(
            owner in valid_owner_name(),
            repo in valid_repo_name(),
            prefix_ws in "\\s*",
            suffix_ws in "\\s*"
        ) {
            let url = format!("{prefix_ws}git@github.com:{owner}/{repo}.git{suffix_ws}");
            let result = GitRepoInfo::parse_github_url(&url);

            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert_eq!(info.owner, Owner::new(owner));
            prop_assert_eq!(info.repo, Repo::new(repo));
        }
    }

    #[test]
    fn test_very_long_url() {
        let long_owner = "a".repeat(1000);
        let long_repo = "b".repeat(1000);
        let url = format!("git@github.com:{long_owner}/{long_repo}.git");

        let result = GitRepoInfo::parse_github_url(&url);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.owner, Owner::new(long_owner));
        assert_eq!(info.repo, Repo::new(long_repo));
    }

    #[test]
    fn test_unicode_in_names() {
        let result = GitRepoInfo::parse_github_url("git@github.com:café/señor.git");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.owner, Owner::new("café"));
        assert_eq!(info.repo, Repo::new("señor"));
    }
}
