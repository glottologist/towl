//! Interactive terminal UI for browsing, filtering, and selecting TODO comments.
//!
//! Provides a full-screen TUI built on [`ratatui`] with keyboard navigation,
//! type filtering, sorting, source-code peeking, and GitHub issue creation.
//! Launch with [`run`].

pub mod app;
pub mod error;
pub mod input;
pub mod render;

pub use error::TowlTuiError;

use std::io;
use std::path::{Path, PathBuf};

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::comment::todo::TodoComment;
use crate::config::GitHubConfig;
use crate::github::types::CreatedIssue;
use crate::github::GitHubClient;
use crate::processor::Processor;

use self::app::{App, AppMode};
use self::input::Action;

enum CreationEvent {
    Phase(String),
    Progress { current: usize, total: usize },
    Error(String),
    IssueCreated(CreatedIssue),
    Finished,
}

/// Launches the interactive TUI for browsing and acting on TODO comments.
///
/// Takes ownership of the terminal, entering raw mode and an alternate screen.
/// Terminal state is always restored on exit, even on error.
///
/// # Errors
/// Returns `TowlTuiError` on terminal I/O failures.
pub fn run(
    todos: Vec<TodoComment>,
    github_config: &GitHubConfig,
    repo_root: &Path,
) -> Result<(), TowlTuiError> {
    let mut app = App::new(todos);

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app, github_config, repo_root);

    terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    github_config: &GitHubConfig,
    repo_root: &Path,
) -> Result<(), TowlTuiError> {
    let tick_rate = std::time::Duration::from_millis(100);
    let mut creation_rx: Option<mpsc::Receiver<CreationEvent>> = None;

    loop {
        terminal.draw(|f| render::draw(f, app))?;

        if let Some(rx) = &mut creation_rx {
            if drain_creation_events(app, rx) {
                creation_rx = None;
            }
        }

        if matches!(app.mode(), AppMode::Creating(_)) && creation_rx.is_none() {
            let (tx, rx) = mpsc::channel(32);
            creation_rx = Some(rx);

            if let Some(delete_todos) = app.take_pending_delete() {
                let root = repo_root.to_path_buf(); // clone: spawned task needs owned path
                tokio::spawn(async move {
                    delete_todos_task(delete_todos, root, tx).await;
                });
            } else {
                let todos = app.selected_todos();
                let config = github_config.clone(); // clone: spawned task needs owned config
                let root = repo_root.to_path_buf(); // clone: spawned task needs owned path
                tokio::spawn(async move {
                    create_issues_task(todos, config, root, tx).await;
                });
            }
            continue;
        }

        match input::handle_input(app, tick_rate)? {
            Action::Quit => return Ok(()),
            Action::Continue => {}
        }
    }
}

fn drain_creation_events(app: &mut App, rx: &mut mpsc::Receiver<CreationEvent>) -> bool {
    loop {
        match rx.try_recv() {
            Ok(event) => match event {
                CreationEvent::Phase(phase) => app.set_creation_phase(phase),
                CreationEvent::Progress { current, total } => {
                    app.set_creation_progress(current, total);
                }
                CreationEvent::Error(msg) => app.push_creation_error(msg),
                CreationEvent::IssueCreated(issue) => app.push_created_issue(issue),
                CreationEvent::Finished => {
                    app.finish_creating();
                    return true;
                }
            },
            Err(mpsc::error::TryRecvError::Empty) => return false,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                if matches!(app.mode(), AppMode::Creating(_)) {
                    app.push_creation_error("Background task disconnected".to_string());
                    app.finish_creating();
                }
                return true;
            }
        }
    }
}

async fn send_event(tx: &mpsc::Sender<CreationEvent>, event: CreationEvent) {
    if let Err(e) = tx.send(event).await {
        tracing::debug!("Failed to send creation event: {e}");
    }
}

async fn init_github_client(
    config: &GitHubConfig,
    tx: &mpsc::Sender<CreationEvent>,
) -> Option<GitHubClient> {
    send_event(tx, CreationEvent::Phase("Initializing client...".into())).await;

    let mut client = match GitHubClient::new(config) {
        Ok(c) => c,
        Err(e) => {
            send_event(tx, CreationEvent::Error(format!("Client init: {e}"))).await;
            return None;
        }
    };

    send_event(
        tx,
        CreationEvent::Phase("Loading existing issues...".into()),
    )
    .await;

    if let Err(e) = client.load_existing_issues().await {
        send_event(tx, CreationEvent::Error(format!("Load existing: {e}"))).await;
        return None;
    }

    Some(client)
}

async fn create_issues_task(
    todos: Vec<TodoComment>,
    github_config: GitHubConfig,
    repo_root: PathBuf,
    tx: mpsc::Sender<CreationEvent>,
) {
    let Some(mut client) = init_github_client(&github_config, &tx).await else {
        send_event(&tx, CreationEvent::Finished).await;
        return;
    };

    let total = todos.len();
    let mut replacements = Vec::new();

    for (i, todo) in todos.into_iter().enumerate() {
        send_event(
            &tx,
            CreationEvent::Phase(format!("Creating issue {}/{}...", i + 1, total)),
        )
        .await;
        send_event(
            &tx,
            CreationEvent::Progress {
                current: i + 1,
                total,
            },
        )
        .await;

        if client.issue_exists(&todo) {
            continue;
        }

        match client.create_issue(&todo).await {
            Ok(issue) => {
                send_event(&tx, CreationEvent::IssueCreated(issue.clone())).await; // clone: send copy to UI, keep for replacement
                replacements.push((todo, issue));
            }
            Err(e) => {
                send_event(&tx, CreationEvent::Error(e.to_string())).await;
            }
        }
    }

    if !replacements.is_empty() {
        send_event(
            &tx,
            CreationEvent::Phase("Replacing TODOs in files...".into()),
        )
        .await;
        let result = Processor::replace_todos(&repo_root, &replacements).await;
        for (path, err) in &result.errors {
            send_event(
                &tx,
                CreationEvent::Error(format!("{}: {err}", path.display())),
            )
            .await;
        }
    }

    send_event(&tx, CreationEvent::Finished).await;
}

async fn delete_todos_task(
    todos: Vec<TodoComment>,
    repo_root: PathBuf,
    tx: mpsc::Sender<CreationEvent>,
) {
    send_event(
        &tx,
        CreationEvent::Phase("Deleting invalid TODOs...".into()),
    )
    .await;

    let canonical_root = match repo_root.canonicalize() {
        Ok(d) => d,
        Err(e) => {
            send_event(
                &tx,
                CreationEvent::Error(format!(
                    "Cannot resolve repository root {}: {e}",
                    repo_root.display()
                )),
            )
            .await;
            send_event(&tx, CreationEvent::Finished).await;
            return;
        }
    };

    let mut by_file: std::collections::HashMap<&std::path::Path, Vec<(usize, &str)>> =
        std::collections::HashMap::new();
    for todo in &todos {
        by_file
            .entry(todo.file_path.as_path())
            .or_default()
            .push((todo.line_number, todo.original_text.as_str()));
    }

    let total = by_file.len();
    for (i, (path, mut line_entries)) in by_file.into_iter().enumerate() {
        send_event(
            &tx,
            CreationEvent::Progress {
                current: i + 1,
                total,
            },
        )
        .await;

        line_entries.sort_unstable_by_key(|&(line, _)| line);
        line_entries.dedup_by_key(|&mut (line, _)| line);

        if let Err(msg) = delete_file_todos(path, &line_entries, &canonical_root).await {
            send_event(&tx, CreationEvent::Error(msg)).await;
        }
    }

    send_event(&tx, CreationEvent::Finished).await;
}

/// Deletes the given `(line, original_text)` entries from one file, after
/// validating the path against `canonical_root` and that the lines still
/// match the scanned text.
async fn delete_file_todos(
    path: &std::path::Path,
    line_entries: &[(usize, &str)],
    canonical_root: &std::path::Path,
) -> Result<(), String> {
    let canonical = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !canonical.starts_with(canonical_root) {
        return Err(format!("{}: outside repository root", path.display()));
    }

    let content = tokio::fs::read_to_string(&canonical)
        .await
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();

    let changed_line = line_entries.iter().find(|&&(line, original)| {
        line.checked_sub(1)
            .and_then(|idx| lines.get(idx))
            .map_or(true, |current| *current != original)
    });
    if let Some(&(line, _)) = changed_line {
        return Err(format!(
            "{}:{line}: file changed since the scan, skipping",
            path.display()
        ));
    }

    let line_set: std::collections::HashSet<usize> =
        line_entries.iter().map(|&(line, _)| line).collect();
    let filtered: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !line_set.contains(&(i + 1)))
        .map(|(_, line)| *line)
        .collect();
    // str::lines strips \r; join with the file's own ending so CRLF files are
    // not silently converted to LF
    let line_ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut new_content = filtered.join(line_ending);
    if content.ends_with('\n') {
        new_content.push_str(line_ending);
    }

    crate::atomic_write(&canonical, new_content.as_bytes())
        .await
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::test_support::TestTodoBuilder;

    async fn run_delete(todos: Vec<TodoComment>, repo_root: PathBuf) -> (Vec<String>, bool) {
        let (tx, mut rx) = mpsc::channel(32);
        delete_todos_task(todos, repo_root, tx).await;

        let mut errors = Vec::new();
        let mut finished = false;
        while let Ok(event) = rx.try_recv() {
            match event {
                CreationEvent::Error(msg) => errors.push(msg),
                CreationEvent::Finished => finished = true,
                _ => {}
            }
        }
        (errors, finished)
    }

    fn delete_target(file: &std::path::Path, line: usize, original: &str) -> TodoComment {
        TestTodoBuilder::new()
            .file_path(file)
            .line_number(line)
            .original_text(original)
            .build()
    }

    #[tokio::test]
    async fn test_delete_removes_line_within_repo_root() {
        let temp = tempfile::TempDir::new().unwrap();
        let file = temp.path().join("x.rs");
        std::fs::write(&file, "fn main() {\n// TODO: gone\n}\n").unwrap();

        let todo = delete_target(&file, 2, "// TODO: gone");
        let (errors, finished) = run_delete(vec![todo], temp.path().to_path_buf()).await;

        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert!(finished);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "fn main() {\n}\n");
    }

    #[tokio::test]
    async fn test_delete_works_when_repo_root_differs_from_cwd() {
        // the scanned root, not the process cwd, is the validation boundary
        let temp = tempfile::TempDir::new().unwrap();
        assert_ne!(
            std::env::current_dir().unwrap().canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
        let file = temp.path().join("x.rs");
        std::fs::write(&file, "// TODO: gone\nkeep\n").unwrap();

        let todo = delete_target(&file, 1, "// TODO: gone");
        let (errors, _) = run_delete(vec![todo], temp.path().to_path_buf()).await;

        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "keep\n");
    }

    #[tokio::test]
    async fn test_delete_rejects_file_outside_repo_root() {
        let root = tempfile::TempDir::new().unwrap();
        let other = tempfile::TempDir::new().unwrap();
        let file = other.path().join("x.rs");
        let original_content = "// TODO: gone\n";
        std::fs::write(&file, original_content).unwrap();

        let todo = delete_target(&file, 1, "// TODO: gone");
        let (errors, _) = run_delete(vec![todo], root.path().to_path_buf()).await;

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("outside repository root"), "{errors:?}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original_content);
    }

    #[tokio::test]
    async fn test_delete_skips_file_changed_since_scan() {
        let temp = tempfile::TempDir::new().unwrap();
        let file = temp.path().join("x.rs");
        let original_content = "// entirely different line\n";
        std::fs::write(&file, original_content).unwrap();

        let todo = delete_target(&file, 1, "// TODO: gone");
        let (errors, _) = run_delete(vec![todo], temp.path().to_path_buf()).await;

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("changed since the scan"), "{errors:?}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original_content);
    }

    #[tokio::test]
    async fn test_delete_preserves_crlf() {
        let temp = tempfile::TempDir::new().unwrap();
        let file = temp.path().join("x.rs");
        std::fs::write(&file, "keep one\r\n// TODO: gone\r\nkeep two\r\n").unwrap();

        let todo = delete_target(&file, 2, "// TODO: gone");
        let (errors, _) = run_delete(vec![todo], temp.path().to_path_buf()).await;

        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "keep one\r\nkeep two\r\n"
        );
    }
}
