use clap::Parser;
use std::path::PathBuf;
use towl::{
    cli::{Cli, OutputFormat, TowlCommands},
    config::config::{LoadConfig, TowlConfig},
    error::TowlError,
    output::Output,
    scanner::scanner::Scanner,
};
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), TowlError> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    if let Err(_e) = run_cli(cli).await {
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
            context,
            verbose,
        } => scan_todos(path, format, output, todo_type, context, verbose).await,
        TowlCommands::Config { all, validate } => show_config(all, validate).await,
    }
}

async fn init_config(path: PathBuf, _force: bool) -> Result<(), TowlError> {
    TowlConfig::init(&path).await?;
    tracing::info!("Initialized config file at: {}", path.display());
    Ok(())
}

async fn scan_todos(
    path: PathBuf,
    format: OutputFormat,
    output: Option<PathBuf>,
    todo_type: Option<String>,
    _context: bool,
    verbose: bool,
) -> Result<(), TowlError> {
    info!("Scanning {}", path.display());
    let config = TowlConfig::load(None)?;
    info!("Scan config\n{}", config);
    let scanner = Scanner::new(config.parsing)?;

    let todos = scanner.scan(path).await?;

    let filtered_todos: Vec<_> = if let Some(filter_type) = todo_type {
        todos
            .into_iter()
            .filter(|todo| {
                format!("{:?}", todo.todo_type).to_lowercase() == filter_type.to_lowercase()
            })
            .collect()
    } else {
        todos
    };

    if verbose {
        tracing::info!("Found {} TODO comments", filtered_todos.len());
        if let Some(ref output_path) = output {
            tracing::info!("Writing to: {}", output_path.display());
        }
    }

    tracing::info!("Found {} TODO comments", filtered_todos.len());
    let outputter = Output::new(format, output)?;
    let _ = outputter.save(&filtered_todos).await;

    Ok(())
}

async fn show_config(_show_all: bool, _validate: bool) -> Result<(), TowlError> {
    let config = TowlConfig::load(None)?;
    info!("Scan config\n{}", config);
    Ok(())
}
