use std::path::Path;

use ignore::{overrides::OverrideBuilder, WalkBuilder};

use super::error::TowlScannerError;
use super::types::Scanner;

impl Scanner {
    /// See: <https://github.com/glottologist/towl/issues/7>
    ///
    /// # Errors
    /// Returns `TowlScannerError::UnableToWalkFile` if exclude patterns are invalid.
    pub(super) fn build_walker(&self, path: &Path) -> Result<ignore::Walk, TowlScannerError> {
        let mut builder = WalkBuilder::new(path);
        builder.hidden(false).git_ignore(false).follow_links(false);

        if !self.config.exclude_patterns.is_empty() {
            let mut overrides = OverrideBuilder::new(path);
            overrides.add("**")?;
            for pattern in &self.config.exclude_patterns {
                overrides.add(&format!("!{pattern}"))?;
            }
            builder.overrides(overrides.build()?);
        }

        Ok(builder.build())
    }
}
