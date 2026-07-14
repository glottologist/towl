use std::path::Path;

use ignore::{overrides::OverrideBuilder, WalkBuilder};

use super::error::TowlScannerError;
use super::types::Scanner;

impl Scanner {
    /// Builds a walker with gitignore semantics (`.gitignore`, `.ignore`, git
    /// excludes — within git repositories) that also prunes entries matching
    /// the configured `exclude_patterns`.
    ///
    /// # Errors
    /// Returns `TowlScannerError::UnableToWalkFile` if exclude patterns are invalid.
    pub(super) fn build_walker(&self, path: &Path) -> Result<ignore::Walk, TowlScannerError> {
        let mut builder = WalkBuilder::new(path);
        builder.hidden(false).follow_links(false);

        if !self.config.exclude_patterns.is_empty() {
            // Excludes are applied via filter_entry rather than
            // WalkBuilder::overrides: an override whitelist would take
            // precedence over gitignore rules and silently disable them.
            let mut excludes = OverrideBuilder::new(path);
            for pattern in &self.config.exclude_patterns {
                excludes.add(&format!("!{pattern}"))?;
            }
            let excludes = excludes.build()?;

            builder.filter_entry(move |entry| {
                let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
                !matches!(
                    excludes.matched(entry.path(), is_dir),
                    ignore::Match::Ignore(_)
                )
            });
        }

        Ok(builder.build())
    }
}
