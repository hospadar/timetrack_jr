use rusqlite::Connection;

use crate::{cli, TTError};

pub fn export(
    conn: &Connection,
    format: &cli::ExportFormat,
    listen: &bool,
    outfile: &String,
    start_time: &Option<String>,
    end_time: &Option<String>,
) -> Result<(), TTError> {
    todo!()
}
