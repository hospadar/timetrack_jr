use crate::{
    cli,
    db::{self, TimeWindow},
    TTError,
};
use chrono::{DateTime, Datelike};

use rusqlite::Connection;
use std::{io, time::Duration};

fn roll_months<T: chrono::TimeZone>(date: &DateTime<T>, num_months: i32) -> DateTime<T> {
    let mut new_date = date.clone();
    if num_months == 0 {
        return new_date;
    }

    for _ in 0..num_months {
        if num_months < 0 {
            //decrement
            if new_date.month0() == 0 {
                //decrement the year
                new_date = new_date
                    .with_month0(11)
                    .unwrap()
                    .with_year(new_date.year() - 1)
                    .unwrap()
            }
            new_date = new_date.with_month0(new_date.month0() - 1).unwrap();
        } else {
            //increment
            if new_date.month0() == 11 {
                //increment the year
                new_date = new_date
                    .with_month0(0)
                    .unwrap()
                    .with_year(new_date.year() + 1)
                    .unwrap()
            }
            new_date = new_date.with_month0(new_date.month0() + 1).unwrap();
        }
    }

    return new_date;
}

fn time_string_to_tstamp(tstring: &Option<String>) -> Option<i64> {
    match tstring {
        Some(raw_time) => {
            if let Ok(parsed) = chrono_english::parse_date_string(
                raw_time,
                chrono::Local::now(),
                chrono_english::Dialect::Us,
            ) {
                Some(parsed.timestamp())
            } else if let Ok(parsed_duration) = chrono_english::parse_duration(raw_time) {
                let mut parsed_time = chrono::Local::now();
                match parsed_duration {
                    chrono_english::Interval::Seconds(n) => {
                        parsed_time += chrono::Duration::seconds(n as i64)
                    }
                    chrono_english::Interval::Days(n) => {
                        parsed_time += chrono::Duration::days(n as i64)
                    }
                    chrono_english::Interval::Months(n) => {
                        parsed_time = roll_months(&parsed_time, n)
                    }
                }
                return Some(parsed_time.timestamp());
            } else {
                None
            }
        }
        _ => None,
    }
}

fn export_json(
    outfile: &mut Box<dyn std::io::Write>,
    times: Vec<TimeWindow>,
) -> Result<(), TTError> {
    outfile.write_all(serde_json::to_string_pretty(&times)?.as_bytes())?;
    Ok(())
}

fn gen_export(
    conn: &mut Connection,
    format: &cli::ExportFormat,
    outfile: &String,
    start_time: &Option<String>,
    end_time: &Option<String>,
) -> Result<(), TTError> {
    let mut handle: Box<dyn std::io::Write> = Box::new(io::stdout());
    if outfile != "-" {
        handle = Box::new(std::fs::File::create(outfile)?)
    }
    let mut tx = conn.transaction()?;
    //parse and check options
    let start = time_string_to_tstamp(start_time);
    if start_time.is_some() && start.is_none() {
        return Err(TTError::TTError {
            message: "Was unable to parse start-time".to_string(),
        });
    }
    let end = time_string_to_tstamp(end_time);
    if end_time.is_some() && end.is_none() {
        return Err(TTError::TTError {
            message: "was unable to parse end-time".to_string(),
        });
    }
    //fetch times from database
    let times = db::get_times(&mut tx, start, end)?;
    match format {
        cli::ExportFormat::Json => export_json(&mut handle, times)?,
        cli::ExportFormat::Csv => todo!(),
        cli::ExportFormat::Ical => todo!(),
        cli::ExportFormat::Summary => todo!(),
    }
    handle.flush()?;
    Ok(())
}

pub fn export(
    conn: &mut Connection,
    format: &cli::ExportFormat,
    listen: &bool,
    outfile: &String,
    start_time: &Option<String>,
    end_time: &Option<String>,
) -> Result<(), TTError> {
    if *listen {
        let mut last_rowid = conn.last_insert_rowid();
        loop {
            let next_rowid: i64 = conn.last_insert_rowid();
            if last_rowid != next_rowid {
                match gen_export(conn, format, outfile, start_time, end_time) {
                    Err(e) => println!("Could not generate export! Error: {:?}", e),
                    _ => {}
                }
                last_rowid = next_rowid
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    } else {
        return gen_export(conn, format, outfile, start_time, end_time);
    }
}
