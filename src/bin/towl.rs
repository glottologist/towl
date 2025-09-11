use cargo_towl::{
    cli::{Cli, Commands, OutputFormat, TowlCommands},
    error::TowlError,
};
use clap::Parser;
use std::env;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), TowlError> {
    // Initialize logging to stderr to avoid interfering with stdout
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
        Commands::Towl { subcommand } => match subcommand {
            TowlCommands::Init {
                owner,
                repo,
                token,
                force,
            } => init_config(owner, repo, token, force).await,
            TowlCommands::Scan {
                path,
                format,
                todo_type,
                context,
                verbose,
            } => scan_todos(path, format, todo_type, context, verbose).await,
            TowlCommands::Config { all, validate } => show_config(all, validate).await,
        },
    }
}

async fn init_config(
    _owner: String,
    _repo: String,
    _token: Option<String>,
    _force: bool,
) -> Result<(), TowlError> {
    Ok(())
}

async fn scan_todos(
    _path: PathBuf,
    _format: OutputFormat,
    _todo_type: Option<String>,
    _context: bool,
    _verbose: bool,
) -> Result<(), TowlError> {
    Ok(())
}

async fn show_config(_show_all: bool, _validate: bool) -> Result<(), TowlError> {
    Ok(())
}
