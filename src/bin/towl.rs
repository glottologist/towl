use clap::Parser;
use std::path::PathBuf;
use towl::{
    cli::{Cli, OutputFormat, TowlCommands},
    comment::todo::TodoType,
    config::TowlConfig,
    error::TowlError,
    output::Output,
    scanner::Scanner,
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), TowlError> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

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
            format,
            output,
            todo_type,
            verbose,
        } => scan_todos(path, format, output, todo_type, verbose).await,
        TowlCommands::Config => show_config(),
    }
}

async fn init_config(path: PathBuf, force: bool) -> Result<(), TowlError> {
    TowlConfig::init(&path, force).await?;
    tracing::info!("Initialized config file at: {}", path.display());
    Ok(())
}

async fn scan_todos(
    path: PathBuf,
    format: OutputFormat,
    output: Option<PathBuf>,
    todo_type: Option<TodoType>,
    verbose: bool,
) -> Result<(), TowlError> {
    info!("Scanning {}", path.display());
    let config = TowlConfig::load(None)?;
    info!("Scan config\n{}", config);
    let scanner = Scanner::new(config.parsing)?;

    let scan_result = scanner.scan(path).await?;

    if scan_result.all_files_failed() {
        eprintln!(
            "Warning: all {} scanned files failed. No TODOs could be collected.",
            scan_result.files_errored
        );
    }

    let filtered_todos: Vec<_> = if let Some(filter_type) = todo_type {
        scan_result
            .todos
            .into_iter()
            .filter(|todo| todo.todo_type == filter_type)
            .collect()
    } else {
        scan_result.todos
    };

    if verbose {
        tracing::info!(
            "Found {} TODO comments ({} files scanned, {} skipped, {} errored in {:?})",
            filtered_todos.len(),
            scan_result.files_scanned,
            scan_result.files_skipped,
            scan_result.files_errored,
            scan_result.duration,
        );
        if let Some(ref output_path) = output {
            tracing::info!("Writing to: {}", output_path.display());
        }
    }

    let outputter = Output::new(format, output)?;

    // Intentionally ignore save errors - scanner succeeded, output is best-effort
    match outputter.save(&filtered_todos).await {
        Ok(()) => {
            if verbose {
                info!(
                    "Successfully saved {} todos to output",
                    filtered_todos.len()
                );
            }
            Ok(())
        }
        Err(e) => {
            error!("Failed to save output: {}", e);
            eprintln!("Warning: Failed to save output: {e}");
            eprintln!("Scan completed successfully, but output could not be written.");
            Ok(())
        }
    }
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

        let filtered_todos: Vec<_> = if let Some(filter_type) = todo_type {
            todos
                .into_iter()
                .filter(|todo| todo.todo_type == filter_type)
                .collect()
        } else {
            todos
        };

        assert_eq!(filtered_todos.len(), expected_count);
    }
}
