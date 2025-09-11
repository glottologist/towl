use clap::{Parser, Subcommand, ValueEnum, Args};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "cargo-towl",
    bin_name = "cargo",
    author,
    version,
    about = "Convert TODO comments to GitHub issues automatically",
    long_about = "tOwl - A cargo plugin that scans your codebase for TODO comments and converts them into GitHub issues"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run tOwl commands
    Towl {
        #[command(subcommand)]
        subcommand: TowlCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum TowlCommands {
    /// Initialize a new .towl.toml configuration file
    Init {
        /// GitHub repository owner/organization
        #[arg(long)]
        owner: String,

        /// GitHub repository name
        #[arg(long)]
        repo: String,

        /// GitHub personal access token (can also be set via GITHUB_TOKEN env var)
        #[arg(long, env = "GITHUB_TOKEN")]
        token: Option<String>,

        /// Force overwrite existing config
        #[arg(long, short = 'f')]
        force: bool,
    },

    /// Scan for TODO comments in the codebase
    Scan {
        /// Path to scan (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format
        #[arg(long, short = 'f', value_enum, default_value = "table")]
        format: OutputFormat,

        /// Filter by TODO type
        #[arg(long, short = 't')]
        todo_type: Option<String>,

        /// Show context lines
        #[arg(long, short = 'c')]
        context: bool,

        /// Verbose output
        #[arg(long, short = 'v')]
        verbose: bool,
    },


    /// Show tOwl configuration
    Config {
        /// Show full configuration including defaults
        #[arg(long, short = 'a')]
        all: bool,

        /// Validate configuration
        #[arg(long)]
        validate: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Markdown,
}
