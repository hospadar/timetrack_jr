use rusqlite::Connection;

use crate::TTError;

pub fn start_timing(conn: &Connection, category_name: &String) -> Result<(), TTError> {
    todo!()
}

pub fn stop_timing(conn: &Connection) -> Result<(), TTError> {
    todo!()
}

pub fn amend_time(
    conn: &Connection,
    time_id: &u64,
    start_time: &Option<String>,
    end_time: &Option<String>,
    category_name: &Option<String>,
) -> Result<(), TTError> {
    todo!()
}
