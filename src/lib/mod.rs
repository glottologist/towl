pub mod cli;
pub mod comment;
pub mod config;
pub mod error;
pub mod output;
pub mod parser;
pub mod scanner;

use std::path::Path;

pub(crate) fn contains_path_traversal(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    proptest! {
        #[test]
        fn prop_detects_traversal_when_present(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
            insert_pos in 0usize..5,
        ) {
            let mut parts: Vec<String> = components;
            let pos = insert_pos.min(parts.len());
            parts.insert(pos, "..".to_string());

            let path: PathBuf = parts.iter().collect();
            prop_assert!(
                contains_path_traversal(&path),
                "Should detect traversal in: {:?}",
                path
            );
        }

        #[test]
        fn prop_accepts_safe_paths(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
        ) {
            let path: PathBuf = components.iter().collect();
            prop_assert!(
                !contains_path_traversal(&path),
                "Should accept safe path: {:?}",
                path
            );
        }

        #[test]
        fn prop_single_dotdot_always_detected(
            prefix in "[a-zA-Z0-9_-]{1,10}",
            suffix in "[a-zA-Z0-9_-]{1,10}",
        ) {
            let path = PathBuf::from(&prefix).join("..").join(&suffix);
            prop_assert!(contains_path_traversal(&path));
        }
    }
}
