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
            let todos = app.selected_todos();
            let config = github_config.clone(); // clone: spawned task needs owned config
            let root = repo_root.to_path_buf(); // clone: spawned task needs owned path
            tokio::spawn(async move {
                create_issues_task(todos, config, root, tx).await;
            });
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
                    app.push_creation_error("Background task disconnected".to_string()); // clone: owned String for error message
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
                send_event(&tx, CreationEvent::Error(e.to_string())).await; // clone: owned String for channel send
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
