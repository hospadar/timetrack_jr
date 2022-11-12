/*
This file is part of Timetrack Jr.
Timetrack Jr. is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
Timetrack Jr. is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
You should have received a copy of the GNU General Public License along with Timetrack Jr. If not, see <https://www.gnu.org/licenses/>. 
*/
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
        Commands::StartTiming { category_name, notify } => log::start_timing(conn, category_name, notify),
        Commands::StopTiming {notify} => log::stop_timing(conn, notify),
        Commands::AmendTime {
            time_id,
            start_time,
            end_time,
            category,
        } => log::amend_time(conn, time_id, start_time, end_time, category),
        Commands::DeleteTime { time_id } => log::delete_time(conn, time_id),
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
