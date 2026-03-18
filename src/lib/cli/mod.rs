//! Command-line interface definitions using [`clap`].

use crate::comment::todo::TodoType;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Top-level CLI parser. Use [`Cli::command`] to access the chosen subcommand.
#[derive(Debug, Parser)]
#[command(
    name = "towl",
    author,
    version,
    about = "Todo Owl - watches over your project's source code for TODO comments",
    long_about = "Todo Owl (tOwl) - Scans your codebase for TODO comments and outputs them in various formats"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: TowlCommands,
}

#[derive(Debug, Subcommand)]
pub enum TowlCommands {
    /// Initialize a new .towl.toml configuration file
    Init {
        /// Path for the configuration file
        #[arg(long, short = 'p', default_value = ".towl.toml")]
        path: PathBuf,

        /// Overwrite existing configuration file
        #[arg(long, short = 'F')]
        force: bool,
    },

    /// Scan for TODO comments in source code
    Scan {
        /// Directory to scan for TODO comments
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Disable interactive TUI mode (for CI/scripting)
        #[arg(long, short = 'N')]
        non_interactive: bool,

        /// Output format for non-interactive mode
        #[arg(long, short = 'f', value_enum, default_value = "terminal")]
        format: OutputFormat,

        /// Write output to a file instead of stdout
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Filter results by TODO type
        #[arg(long, short = 't', value_enum)]
        todo_type: Option<TodoType>,

        /// Show detailed scan statistics
        #[arg(long, short = 'v')]
        verbose: bool,

        /// Create GitHub issues for found TODOs
        #[arg(long, short = 'g')]
        github: bool,

        /// Preview GitHub issues without creating them
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Analyse TODOs with AI to validate relevance
        #[arg(long)]
        ai: bool,
    },

    /// Display the current configuration
    Config,
}

/// Output format for non-interactive scan results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Toml,
    Markdown,
    Terminal,
}
