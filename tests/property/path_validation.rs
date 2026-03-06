use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;
use towl::{config::TowlConfig, error::TowlError, scanner::Scanner};

// Property test strategies for generating various path types
prop_compose! {
    fn arbitrary_path_component()(s in "[a-zA-Z0-9_-]{1,20}") -> String {
        s
    }
}

prop_compose! {
    fn arbitrary_safe_path()(
        components in prop::collection::vec(arbitrary_path_component(), 1..5)
    ) -> PathBuf {
        components.iter().collect()
    }
}

prop_compose! {
    fn arbitrary_filename()(
        name in "[a-zA-Z0-9_-]{1,20}",
        ext in prop::option::of("[a-z]{1,5}")
    ) -> String {
        match ext {
            Some(extension) => format!("{name}.{extension}"),
            None => name,
        }
    }
}

proptest! {
    #[test]
    fn init_config_handles_arbitrary_safe_paths(path in arbitrary_safe_path()) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let full_path = temp_dir.path().join(&path).join("towl.toml");

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            towl::config::TowlConfig::init(&full_path, true).await
        });

        // If successful, config file should exist
        if result.is_ok() {
            prop_assert!(full_path.exists());
        }
    }

    #[test]
    fn scan_todos_handles_arbitrary_safe_paths(path in arbitrary_safe_path()) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let scan_path = temp_dir.path().join(&path);

        std::fs::create_dir_all(&scan_path).ok();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let config = crate::fixtures::mock_towl_config();
            let scanner = Scanner::new(config.parsing)
                .map_err(TowlError::Scanner)?;
            scanner.scan(scan_path).await.map(|r| r.todos).map_err(TowlError::Scanner)
        });

        prop_assert!(result.is_ok(), "Scan of created directory should succeed: {:?}", result.err());
    }

    #[test]
    fn output_filename_generation_is_safe(filename in arbitrary_filename()) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let output_path = temp_dir.path().join(&filename);

        // Property: Generated paths should not contain dangerous sequences
        let path_str = output_path.to_string_lossy();
        prop_assert!(!path_str.contains(".."));
        prop_assert!(!path_str.contains("//"));

        // Property: Path should be constructible and not cause issues
        prop_assert!(output_path.file_name().is_some());
    }

    #[test]
    fn path_traversal_protection_invariant(
        base_components in prop::collection::vec(arbitrary_path_component(), 1..3),
        malicious_suffix in "(\\.\\./){1,5}[a-zA-Z0-9_-]{1,10}"
    ) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let mut base_path = temp_dir.path().to_path_buf();
        for component in &base_components {
            base_path = base_path.join(component);
        }

        let malicious_path = base_path.join(&malicious_suffix);

        // Property: Paths with ".." components should contain ParentDir components
        let has_parent_dir = malicious_path.components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
        let path_str = malicious_path.to_string_lossy();

        if path_str.contains("..") {
            prop_assert!(has_parent_dir, "Path traversal should be detected for: {}", path_str);
        }

        // If canonicalization succeeds, verify it stays within the temp directory
        if let Ok(canonical) = malicious_path.canonicalize() {
            let canonical_str = canonical.to_string_lossy();
            let temp_str = temp_dir.path().to_string_lossy();
            prop_assert!(
                canonical_str.starts_with(&*temp_str),
                "Canonical path {} should be under temp dir {}",
                canonical_str,
                temp_str
            );
        }
    }

    #[test]
    fn config_path_robustness(
        components in prop::collection::vec(arbitrary_path_component(), 0..8)
    ) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_path: PathBuf = if components.is_empty() {
            temp_dir.path().join("towl.toml")
        } else {
            let mut path = temp_dir.path().to_path_buf();
            for component in &components {
                path = path.join(component);
            }
            path.join("towl.toml")
        };

        // Create parent directories
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let rt = tokio::runtime::Runtime::new().unwrap();

        // Property: TowlConfig operations should be robust to various path structures
        let init_result = rt.block_on(async {
            TowlConfig::init(&config_path, true).await
        });

        // If init succeeded, loading should also work
        if init_result.is_ok() && config_path.exists() {
            let load_result = TowlConfig::load(Some(&config_path));
            prop_assert!(load_result.is_ok(), "Loading a config we just initialized should succeed: {:?}", load_result);
        }
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("")]
    #[case("../../etc/passwd")]
    #[case("/etc/passwd")]
    fn test_malicious_path_handling(#[case] malicious_input: &str) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let malicious_path = temp_dir.path().join(malicious_input);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let config = crate::fixtures::mock_towl_config();
            let scanner = Scanner::new(config.parsing);
            match scanner {
                Ok(s) => s
                    .scan(malicious_path)
                    .await
                    .map(|r| r.todos)
                    .map_err(TowlError::Scanner),
                Err(e) => Err(TowlError::Scanner(e)),
            }
        });

        if let Ok(todos) = &result {
            for todo in todos {
                assert!(!todo.id.is_empty(), "TODO IDs must never be empty");
                assert!(
                    !todo.file_path.to_string_lossy().contains("/etc/"),
                    "Scan should not access files outside working directory"
                );
            }
        }
    }
}
