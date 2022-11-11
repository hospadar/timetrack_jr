use crate::{db::{self, TimeWindow}, TTError, cli};
use notify_rust::{Notification, Timeout};
use rusqlite::{Connection, Transaction};

fn stop_timing_private(tx: &mut Transaction, notify: &bool) -> Result<(), TTError> {
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

pub fn start_timing(conn: &mut Connection, category_name: &String, notify: &bool) -> Result<(), TTError> {
    let mut tx = conn.transaction()?;
    let categories = db::get_categories(&mut tx)?;
    if !categories.contains(category_name) {
        return Err(TTError::TTError { message: format!("Category '{}' does not exist in the timetrack jr database, use `ttjr add-category` to add it", category_name) });
    }
    let mut last_open:Option<TimeWindow> = None;
    if *notify {
        last_open = db::get_last_open_time(&mut tx)?;
    }
    stop_timing_private(&mut tx, notify)?;
    db::start_timing(&mut tx, category_name)?;
    tx.commit()?;

    if *notify {
        if let Some(time) = &last_open {
            Notification::new().summary(&format!("Stopped: {}", time.category)).appname("Timetrack Jr.").show()?;
        }
        Notification::new().summary(&format!("Started: {}", category_name)).appname("Timetrack Jr.").show()?;
    }

    return Ok(());
}

pub fn stop_timing(conn: &mut Connection, notify: &bool) -> Result<(), TTError> {
    let mut tx = conn.transaction()?;
    let mut last_open:Option<TimeWindow> = None;
    if *notify {
        last_open = db::get_last_open_time(&mut tx)?;
    }
    stop_timing_private(&mut tx, notify)?;
    tx.commit()?;
    if *notify {
        if let Some(time) = &last_open {
            Notification::new().summary(&format!("Stopped: {}", time.category)).appname("Timetrack Jr.").show()?;
        }
    }
    return Ok(());
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
    if let Some(start) = cli::time_string_to_tstamp(start_time) {
        time.start_time = start;
    }
    if let Some(end) = cli::time_string_to_tstamp(end_time) {
        time.end_time = Some(end);
    }
    if let Some(category) = category_name {
        time.category = category.clone();
    }

    db::upsert_time(&mut tx, time)?;
    tx.commit()?;
    Ok(())
}

pub fn delete_time(
    conn: &mut Connection,
    time_id: &i64,
)-> Result<(), TTError> {
    let mut tx = conn.transaction()?;
    let did_delete = db::delete_time(&mut tx, &time_id)?;
    tx.commit()?;
    if did_delete == 0 {
        Err(TTError::TTError { message: "Invalid time ID".to_string() })
    } else {
        Ok(())
    }

}