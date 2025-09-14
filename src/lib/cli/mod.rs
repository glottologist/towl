use clap::{ Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::config::config::DEFAULT_CONFIG_PATH;

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
        /// Path to the config file (defaults to .towl.toml)
        #[arg(long, short = 'p', default_value = DEFAULT_CONFIG_PATH)]
        path: PathBuf,

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
