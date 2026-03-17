use clap::Parser;
use std::path::{Path, PathBuf};
use towl::{
    cli::{Cli, OutputFormat, TowlCommands},
    comment::todo::{TodoComment, TodoType},
    config::{GitHubConfig, TowlConfig},
    error::TowlError,
    github::{CreatedIssue, GitHubClient},
    output::Output,
    processor::{Processor, ProcessorResult},
    scanner::{ScanResult, Scanner},
};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), TowlError> {
    let cli = Cli::parse();

    let suppress_tracing = matches!(
        cli.command,
        TowlCommands::Scan {
            non_interactive: false,
            ..
        }
    );

    if !suppress_tracing {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .init();
    }

    if let Err(e) = run_cli(cli).await {
        error!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}

async fn run_cli(cli: Cli) -> Result<(), TowlError> {
    match cli.command {
        TowlCommands::Init { path, force } => init_config(path, force).await,
        TowlCommands::Scan {
            path,
            non_interactive,
            format,
            output,
            todo_type,
            verbose,
            github,
            dry_run,
        } => {
            if non_interactive {
                scan_todos(path, format, output, todo_type, verbose, github, dry_run).await
            } else {
                run_interactive(path).await
            }
        }
        TowlCommands::Config => show_config(),
    }
}

async fn init_config(path: PathBuf, force: bool) -> Result<(), TowlError> {
    TowlConfig::init(&path, force).await?;
    info!("Initialized config file at: {}", path.display());
    Ok(())
}

async fn load_and_scan(path: &Path) -> Result<(GitHubConfig, ScanResult), TowlError> {
    info!("Scanning {}", path.display());
    let config = TowlConfig::load(None)?;
    info!("Scan config\n{}", config);
    let scanner = Scanner::new(config.parsing)?;
    let scan_result = scanner.scan(path.to_path_buf()).await?; // clone: scan takes owned PathBuf

    if scan_result.all_files_failed() {
        eprintln!(
            "Warning: all {} scanned files failed. No TODOs could be collected.",
            scan_result.files_errored
        );
    }

    Ok((config.github, scan_result))
}

async fn scan_todos(
    path: PathBuf,
    format: OutputFormat,
    output: Option<PathBuf>,
    todo_type: Option<TodoType>,
    verbose: bool,
    github: bool,
    dry_run: bool,
) -> Result<(), TowlError> {
    let (github_config, scan_result) = load_and_scan(&path).await?;

    let files_scanned = scan_result.files_scanned;
    let files_skipped = scan_result.files_skipped;
    let files_errored = scan_result.files_errored;
    let duration = scan_result.duration;
    let filtered_todos = filter_todos(scan_result.todos, todo_type);

    if verbose {
        log_scan_verbose(
            &filtered_todos,
            files_scanned,
            files_skipped,
            files_errored,
            duration,
            output.as_ref(),
        );
    }

    save_output(format, output, &filtered_todos, verbose).await?;

    if github {
        create_github_issues(&path, &github_config, filtered_todos, dry_run).await?;
    }

    Ok(())
}

async fn create_github_issues(
    repo_root: &Path,
    github_config: &GitHubConfig,
    todos: Vec<TodoComment>,
    dry_run: bool,
) -> Result<(), TowlError> {
    if todos.is_empty() {
        debug!("No TODOs found, skipping GitHub issue creation");
        return Ok(());
    }

    if dry_run {
        report_dry_run(&todos);
        return Ok(());
    }

    let mut client = GitHubClient::new(github_config)?;
    client.load_existing_issues().await?;

    let (replacements, skipped, failed) = submit_issues(&mut client, todos).await;
    let created = replacements.len();
    let result = Processor::replace_todos(repo_root, &replacements).await;

    report_github_results(created, skipped, failed, &result);

    Ok(())
}

enum IssueOutcome {
    Created(Box<TodoComment>, CreatedIssue),
    Skipped,
    Failed,
}

async fn try_create_issue(client: &mut GitHubClient, todo: TodoComment) -> IssueOutcome {
    if client.issue_exists(&todo) {
        debug!("Skipping duplicate: {}", todo.description);
        return IssueOutcome::Skipped;
    }

    match client.create_issue(&todo).await {
        Ok(issue) => {
            info!("Created issue #{}: {}", issue.number, issue.title);
            IssueOutcome::Created(Box::new(todo), issue)
        }
        Err(e) => {
            warn!("Failed to create issue for {}: {}", todo.description, e);
            IssueOutcome::Failed
        }
    }
}

async fn submit_issues(
    client: &mut GitHubClient,
    todos: Vec<TodoComment>,
) -> (Vec<(TodoComment, CreatedIssue)>, usize, usize) {
    let mut replacements = Vec::new();
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for todo in todos {
        match try_create_issue(client, todo).await {
            IssueOutcome::Created(todo, issue) => replacements.push((*todo, issue)),
            IssueOutcome::Skipped => skipped += 1,
            IssueOutcome::Failed => failed += 1,
        }
    }

    (replacements, skipped, failed)
}

fn report_github_results(created: usize, skipped: usize, failed: usize, result: &ProcessorResult) {
    eprintln!("GitHub: {created} issues created, {skipped} skipped (duplicate), {failed} failed");
    eprintln!(
        "Processor: {} files modified, {} TODOs replaced, {} errors",
        result.files_modified,
        result.todos_replaced,
        result.errors.len()
    );

    for (path, err) in &result.errors {
        warn!("Replacement error in {}: {}", path.display(), err);
    }
}

fn report_dry_run(todos: &[TodoComment]) {
    eprintln!("Dry run: would create {} GitHub issues:", todos.len());
    for todo in todos {
        eprintln!(
            "  - [{}] {} ({}:{})",
            todo.todo_type,
            todo.description.trim(),
            todo.file_path.display(),
            todo.line_number
        );
    }
}

async fn run_interactive(path: PathBuf) -> Result<(), TowlError> {
    let (github_config, scan_result) = load_and_scan(&path).await?;

    if scan_result.todos.is_empty() {
        eprintln!("No TODOs found.");
        return Ok(());
    }

    towl::tui::run(scan_result.todos, &github_config, &path)?;

    Ok(())
}

fn filter_todos(todos: Vec<TodoComment>, todo_type: Option<TodoType>) -> Vec<TodoComment> {
    if let Some(filter_type) = todo_type {
        todos
            .into_iter()
            .filter(|todo| todo.todo_type == filter_type)
            .collect()
    } else {
        todos
    }
}

fn log_scan_verbose(
    filtered_todos: &[TodoComment],
    files_scanned: usize,
    files_skipped: usize,
    files_errored: usize,
    duration: std::time::Duration,
    output: Option<&PathBuf>,
) {
    info!(
        "Found {} TODO comments ({files_scanned} files scanned, {files_skipped} skipped, {files_errored} errored in {duration:?})",
        filtered_todos.len(),
    );
    if let Some(output_path) = output {
        info!("Writing to: {}", output_path.display());
    }
}

async fn save_output(
    format: OutputFormat,
    output: Option<PathBuf>,
    filtered_todos: &[TodoComment],
    verbose: bool,
) -> Result<(), TowlError> {
    let outputter = Output::new(format, output)?;
    outputter.save(filtered_todos).await?;
    if verbose {
        info!(
            "Successfully saved {} todos to output",
            filtered_todos.len()
        );
    }
    Ok(())
}

fn show_config() -> Result<(), TowlError> {
    let config = TowlConfig::load(None)?;
    info!("Scan config\n{}", config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use std::path::PathBuf;
    use towl::comment::todo::{TodoComment, TodoType};

    fn create_mock_todo(todo_type: TodoType) -> TodoComment {
        TodoComment {
            id: "test-id".to_string(),
            todo_type,
            file_path: PathBuf::from("test.rs"),
            line_number: 1,
            column_start: 0,
            column_end: 0,
            original_text: "// TODO: test comment".to_string(),
            description: "test comment".to_string(),
            context_lines: vec![],
            function_context: None,
        }
    }

    #[rstest]
    #[case(None, 3)]
    #[case(Some(TodoType::Todo), 1)]
    #[case(Some(TodoType::Fixme), 1)]
    #[case(Some(TodoType::Hack), 1)]
    #[case(Some(TodoType::Bug), 0)]
    fn test_todo_filtering_logic(
        #[case] todo_type: Option<TodoType>,
        #[case] expected_count: usize,
    ) {
        let todos = vec![
            create_mock_todo(TodoType::Todo),
            create_mock_todo(TodoType::Fixme),
            create_mock_todo(TodoType::Hack),
        ];

        let filtered_todos = super::filter_todos(todos, todo_type);

        assert_eq!(filtered_todos.len(), expected_count);
    }
}
