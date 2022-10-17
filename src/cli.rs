use clap::{Parser, Subcommand};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(long, default_value = "ttjr.sqlite3")]
    db_path: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Set up DB and configure options
    Config,
    /// Listen for key events to trigger time tracking
    Listen,
}