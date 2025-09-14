use super::{
    config::{Owner, Repo},
    error::TowlConfigError,
};
use git2::Repository;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GitRepoInfo {
    pub owner: Owner,
    pub repo: Repo,
}

impl GitRepoInfo {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, TowlConfigError> {
        let repo = Repository::discover(path).map_err(|e| TowlConfigError::GitRepoNotFound {
            message: format!("Failed to find git repository: {}", e),
        })?;

        let remote =
            repo.find_remote("origin")
                .map_err(|e| TowlConfigError::GitRemoteNotFound {
                    message: format!("Failed to find 'origin' remote: {}", e),
                })?;

        let url = remote
            .url()
            .ok_or_else(|| TowlConfigError::GitRemoteNotFound {
                message: "Remote 'origin' has no URL".to_string(),
            })?;

        Self::parse_github_url(url)
    }

    fn parse_github_url(url: &str) -> Result<GitRepoInfo, TowlConfigError> {
        let url = url.trim();

        // Handle SSH URLs (git@github.com:owner/repo.git)
        if url.starts_with("git@github.com:") {
            let path = url
                .strip_prefix("git@github.com:")
                .unwrap()
                .trim_end_matches(".git");

            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() != 2 {
                return Err(TowlConfigError::GitInvalidUrl {
                    url: url.to_string(),
                    message: "Invalid SSH URL format".to_string(),
                });
            }

            return Ok(GitRepoInfo {
                owner: Owner(parts[0].to_string()),
                repo: Repo(parts[1].to_string()),
            });
        }

        // Handle HTTPS URLs (https://github.com/owner/repo.git)
        if url.starts_with("https://github.com/") {
            let path = url
                .strip_prefix("https://github.com/")
                .unwrap()
                .trim_end_matches(".git");

            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() != 2 {
                return Err(TowlConfigError::GitInvalidUrl {
                    url: url.to_string(),
                    message: "Invalid HTTPS URL format".to_string(),
                });
            }

            return Ok(GitRepoInfo {
                owner: Owner(parts[0].to_string()),
                repo: Repo(parts[1].to_string()),
            });
        }

        Err(TowlConfigError::GitInvalidUrl {
            url: url.to_string(),
            message: "URL is not a GitHub repository".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_url() {
        let info = GitRepoInfo::parse_github_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(info.owner, Owner("owner"));
        assert_eq!(info.repo, Repo("repo"));
    }

    #[test]
    fn test_parse_https_url() {
        let info = GitRepoInfo::parse_github_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(info.owner, Owner("owner"));
        assert_eq!(info.repo, Repo("repo"));
    }

    #[test]
    fn test_parse_ssh_url_without_git_extension() {
        let info = GitRepoInfo::parse_github_url("git@github.com:owner/repo").unwrap();
        assert_eq!(info.owner, Owner("owner"));
        assert_eq!(info.repo, Repo("repo"));
    }

    #[test]
    fn test_invalid_url() {
        assert!(GitRepoInfo::parse_github_url("https://gitlab.com/owner/repo").is_err());
    }
}
