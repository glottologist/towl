use crate::comment::todo::TodoComment;

pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
pub const MAX_TODO_COUNT: usize = 10_000;
pub const MAX_TOTAL_TODO_COUNT: usize = 100_000;
pub const MAX_FILES_SCANNED: usize = 100_000;

/// Structured result from a scan operation, distinguishing "no TODOs found"
/// from "all files failed to scan".
#[derive(Debug)]
pub struct ScanResult {
    pub todos: Vec<TodoComment>,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub files_errored: usize,
    pub duration: std::time::Duration,
}

impl ScanResult {
    #[must_use]
    pub const fn all_files_failed(&self) -> bool {
        self.files_scanned == 0 && self.files_errored > 0
    }

    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.todos.is_empty() && self.files_errored == 0
    }
}
