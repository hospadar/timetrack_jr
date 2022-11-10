use crate::cli::{Cli, Commands};
use crate::TTError;
use rusqlite::Connection;

use self::config::unset_option;

mod config;
mod export;
mod log;

pub fn execute(cli: &Cli, conn: &mut Connection) -> Result<(), TTError> {
    match &cli.command {
        Commands::ShowConfig => config::show(conn),
        Commands::AddCategory { category_name } => config::add_category(conn, category_name),
        Commands::DeleteCategory {
            category_name,
            delete_logged_times,
        } => config::delete_category(conn, category_name, delete_logged_times),
        Commands::SetOption {
            option_name,
            option_value,
        } => config::set_option(conn, option_name, option_value),
        Commands::UnsetOption { option_name } => unset_option(conn, option_name),
        Commands::StartTiming { category_name } => log::start_timing(conn, category_name),
        Commands::StopTiming => log::stop_timing(conn),
        Commands::AmendTime {
            time_id,
            start_time,
            end_time,
            category,
        } => log::amend_time(conn, time_id, start_time, end_time, category),
        Commands::Export {
            format,
            listen,
            outfile,
            start_time,
            end_time,
        } => export::export(
            conn,
            format,
            listen,
            &(cli.db_path.clone()).unwrap(),
            outfile,
            start_time,
            end_time,
        ),
    }
}
