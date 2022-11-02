use crate::{db, TTError};
use rusqlite::{Connection, Transaction};

fn stop_timing_private(tx: &mut Transaction) -> Result<(), TTError> {
    let opts = db::get_options(&tx)?;
    let mut done = false;
    if let Some(end) = opts.get("end-of-day") {
        if let Ok(end) = db::parse_time(end) {
            db::end_open_times(tx, end)?;
            done = true;
        }
    }
    if !done {
        db::end_open_times_immediately(tx)?;
    }
    Ok(())
}

pub fn start_timing(conn: &mut Connection, category_name: &String) -> Result<(), TTError> {
    let mut tx = conn.transaction()?;
    let categories = db::get_categories(&mut tx)?;
    if !categories.contains_key(category_name) {
        return Err(TTError::TTError { message: format!("Category '{}' does not exist in the timetrack jr database, use `ttjr add-category` to add it", category_name) });
    }
    stop_timing_private(&mut tx)?;
    db::start_timing(&mut tx, category_name)?;
    return Ok(tx.commit()?);
}

pub fn stop_timing(conn: &mut Connection) -> Result<(), TTError> {
    let mut tx = conn.transaction()?;
    stop_timing_private(&mut tx)?;
    return Ok(tx.commit()?);
}

pub fn amend_time(
    conn: &mut Connection,
    time_id: &i64,
    start_time: &Option<String>,
    end_time: &Option<String>,
    category_name: &Option<String>,
) -> Result<(), TTError> {
    let mut tx = conn.transaction()?;
    let mut time = db::get_time(&tx, time_id.clone())?;
    if let Some(start) = start_time {
        time.start_time = start.parse()?;
    }
    if let Some(end) = end_time {
        time.end_time = Some(end.parse()?);
    }
    if let Some(category) = category_name {
        time.category = category.clone();
    }

    db::upsert_time(&mut tx, time)
}
