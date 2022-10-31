use crate::{cli, db, TTError};
use rusqlite::Connection;

pub fn start_timing(conn: &Connection, category_name: &String) -> Result<(), TTError> {
    todo!()
}

pub fn stop_timing(conn: &mut Connection) -> Result<(), TTError> {
    let mut tx = conn.transaction()?;

    let opts = db::get_options(&tx)?;

    if let (Some(start), Some(end)) = (opts.get("start-of-day"), opts.get("end-of-day")) {
        if let (Ok(start), Ok(end)) = (db::parse_time(start), db::parse_time(end)) {
            if start != end {
                return db::end_open_times(&mut tx, start, end);
            }
        }
    }

    return db::end_open_times_immediately(&mut tx);
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
