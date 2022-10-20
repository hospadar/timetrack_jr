use rusqlite::Connection;
use super::cli::{Cli, Commands, ConfigCommands, LogCommands};

mod config;
mod listen;
mod log;

pub fn execute(cli: &Cli, conn: &Connection) -> Result<(), rusqlite::Error> {
    match &cli.command {
        Commands::Config{config_command} => {
            match config_command {
                ConfigCommands::Show => {
                    config::show(conn)
                }
                ConfigCommands::AddCategory { category_name, set_key_sequence } => {
                    config::add_category(conn, category_name, set_key_sequence)
                },
                ConfigCommands::DeleteCategory { category_name, delete_logged_times } => todo!(),
                ConfigCommands::SetKeySequence { category_name } => todo!(),
                ConfigCommands::SetOption { option_name, option_value } => todo!(),
            }
        },
        Commands::Log { log_command } => todo!(),
        Commands::Listen { ical_file, csv_file, json_file, days_to_export } => todo!(),
    }
}