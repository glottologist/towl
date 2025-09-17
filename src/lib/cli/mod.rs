use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "towl",
    author,
    version,
    about = "Watches over your project's source code for TODO comments",
    long_about = "tOwl - Scans your codebase for TODO comments and outputs them in various formats"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: TowlCommands,
}

#[derive(Debug, Subcommand)]
pub enum TowlCommands {
    Init {
        #[arg(long, short = 'p', default_value = ".towl.toml")]
        path: PathBuf,

        #[arg(long, short = 'f')]
        force: bool,
    },

    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long, short = 'f', value_enum, default_value = "terminal")]
        format: OutputFormat,

        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        #[arg(long, short = 't')]
        todo_type: Option<String>,

        #[arg(long, short = 'c')]
        context: bool,

        #[arg(long, short = 'v')]
        verbose: bool,
    },

    Config {
        #[arg(long, short = 'a')]
        all: bool,

        #[arg(long)]
        validate: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Toml,
    Markdown,
    Terminal,
}
