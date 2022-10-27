use crate::cli::{Cli, Commands, ConfigCommands, LogCommands};
use crate::TTError;
use rusqlite::Connection;

mod config;
mod export;
mod log;

pub fn execute(cli: &Cli, conn: &Connection) -> Result<(), TTError> {
    match &cli.command {
        Commands::Config { config_command } => match config_command {
            ConfigCommands::Show => config::show(conn),
            ConfigCommands::AddCategory { category_name } => {
                config::add_category(conn, category_name)
            }
            ConfigCommands::DeleteCategory {
                category_name,
                delete_logged_times,
            } => config::delete_category(conn, category_name, delete_logged_times),
            ConfigCommands::SetOption {
                option_name,
                option_value,
            } => config::set_option(conn, option_name, option_value),
        },
        Commands::Log { log_command } => match log_command {
            LogCommands::StartTiming { category_name } => log::start_timing(conn, category_name),
            LogCommands::StopTiming => log::stop_timing(conn),
            LogCommands::AmendTime {
                time_id,
                start_time,
                end_time,
                category,
            } => log::amend_time(conn, time_id, start_time, end_time, category),
        },
        Commands::Export {
            format,
            listen,
            outfile,
            start_time,
            end_time,
        } => export::export(conn, format, listen, outfile, start_time, end_time),
    }
}
