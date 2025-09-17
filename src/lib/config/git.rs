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

        if url.starts_with("git@github.com:") {
            let path = url
                .strip_prefix("git@github.com:")
                .ok_or_else(|| TowlConfigError::GitInvalidUrl {
                    url: url.to_string(),
                    message: "Failed to parse SSH URL prefix".to_string(),
                })?
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

        if url.starts_with("https://github.com/") {
            let path = url
                .strip_prefix("https://github.com/")
                .ok_or_else(|| TowlConfigError::GitInvalidUrl {
                    url: url.to_string(),
                    message: "Failed to parse HTTPS URL prefix".to_string(),
                })?
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
    use proptest::prelude::*;
    use rstest::rstest;

    #[rstest]
    #[case("git@github.com:owner/repo.git", "owner", "repo")]
    #[case("git@github.com:my-org/my-repo.git", "my-org", "my-repo")]
    #[case("git@github.com:user123/project456.git", "user123", "project456")]
    #[case("git@github.com:a/b", "a", "b")]
    fn test_parse_ssh_url_variants(
        #[case] url: &str,
        #[case] expected_owner: &str,
        #[case] expected_repo: &str,
    ) {
        let info = GitRepoInfo::parse_github_url(url).unwrap();
        assert_eq!(info.owner, Owner(expected_owner.to_string()));
        assert_eq!(info.repo, Repo(expected_repo.to_string()));
    }

    #[rstest]
    #[case("https://github.com/owner/repo.git", "owner", "repo")]
    #[case("https://github.com/my-org/my-repo.git", "my-org", "my-repo")]
    #[case("https://github.com/user123/project456.git", "user123", "project456")]
    #[case("https://github.com/a/b", "a", "b")]
    fn test_parse_https_url_variants(
        #[case] url: &str,
        #[case] expected_owner: &str,
        #[case] expected_repo: &str,
    ) {
        let info = GitRepoInfo::parse_github_url(url).unwrap();
        assert_eq!(info.owner, Owner(expected_owner.to_string()));
        assert_eq!(info.repo, Repo(expected_repo.to_string()));
    }

    #[rstest]
    #[case("https://gitlab.com/owner/repo.git", "URL is not a GitHub repository")]
    #[case("git@gitlab.com:owner/repo.git", "URL is not a GitHub repository")]
    #[case(
        "https://bitbucket.org/owner/repo.git",
        "URL is not a GitHub repository"
    )]
    #[case("ftp://github.com/owner/repo.git", "URL is not a GitHub repository")]
    #[case("git@github.com:single-part", "Invalid SSH URL format")]
    #[case("git@github.com:too/many/parts", "Invalid SSH URL format")]
    #[case("https://github.com/single-part", "Invalid HTTPS URL format")]
    #[case("https://github.com/too/many/parts", "Invalid HTTPS URL format")]
    fn test_invalid_url_variants(#[case] url: &str, #[case] expected_message: &str) {
        let result = GitRepoInfo::parse_github_url(url);
        assert!(result.is_err());
        if let Err(TowlConfigError::GitInvalidUrl { message, .. }) = result {
            assert_eq!(message, expected_message);
        } else {
            panic!("Expected GitInvalidUrl error");
        }
    }

    #[rstest]
    #[case("   git@github.com:owner/repo.git   ")]
    #[case("\tgit@github.com:owner/repo.git\t")]
    #[case("\n\rgit@github.com:owner/repo.git\r\n")]
    fn test_whitespace_handling(#[case] url: &str) {
        let info = GitRepoInfo::parse_github_url(url).unwrap();
        assert_eq!(info.owner, Owner("owner".to_string()));
        assert_eq!(info.repo, Repo("repo".to_string()));
    }

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
            let url = format!("git@github.com:{}/{}.git", owner, repo);
            let result = GitRepoInfo::parse_github_url(&url);

            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert_eq!(info.owner, Owner(owner));
            prop_assert_eq!(info.repo, Repo(repo));
        }

        #[test]
        fn prop_test_https_url_parsing(
            owner in valid_owner_name(),
            repo in valid_repo_name()
        ) {
            let url = format!("https://github.com/{}/{}.git", owner, repo);
            let result = GitRepoInfo::parse_github_url(&url);

            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert_eq!(info.owner, Owner(owner));
            prop_assert_eq!(info.repo, Repo(repo));
        }

        #[test]
        fn prop_test_invalid_hosts_always_fail(
            host in "[a-z]{3,20}\\.(com|org|net)",
            owner in valid_owner_name(),
            repo in valid_repo_name()
        ) {
            prop_assume!(host != "github.com");

            let ssh_url = format!("git@{}:{}/{}.git", host, owner, repo);
            let https_url = format!("https://{}/{}/{}.git", host, owner, repo);

            prop_assert!(GitRepoInfo::parse_github_url(&ssh_url).is_err());
            prop_assert!(GitRepoInfo::parse_github_url(&https_url).is_err());
        }

        #[test]
        fn prop_test_malformed_paths_fail(
            parts in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 0..2),
        ) {
            prop_assume!(parts.len() != 2); // We want to test invalid cases

            let path = parts.join("/");
            let ssh_url = format!("git@github.com:{}", path);
            let https_url = format!("https://github.com/{}", path);

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
            let url = format!("{}git@github.com:{}/{}.git{}", prefix_ws, owner, repo, suffix_ws);
            let result = GitRepoInfo::parse_github_url(&url);

            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert_eq!(info.owner, Owner(owner));
            prop_assert_eq!(info.repo, Repo(repo));
        }
    }

    #[test]
    fn test_empty_string() {
        let result = GitRepoInfo::parse_github_url("");
        assert!(result.is_err());
    }

    #[test]
    fn test_very_long_url() {
        let long_owner = "a".repeat(1000);
        let long_repo = "b".repeat(1000);
        let url = format!("git@github.com:{}/{}.git", long_owner, long_repo);

        let result = GitRepoInfo::parse_github_url(&url);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.owner, Owner(long_owner));
        assert_eq!(info.repo, Repo(long_repo));
    }

    #[test]
    fn test_unicode_in_names() {
        let result = GitRepoInfo::parse_github_url("git@github.com:café/señor.git");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.owner, Owner("café".to_string()));
        assert_eq!(info.repo, Repo("señor".to_string()));
    }

    #[test]
    fn test_special_characters() {
        let result = GitRepoInfo::parse_github_url("git@github.com:owner-123/repo_456.git");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.owner, Owner("owner-123".to_string()));
        assert_eq!(info.repo, Repo("repo_456".to_string()));
    }
}
