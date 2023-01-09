/*
Copyright 2022 Luke Hospadaruk
This file is part of Timetrack Jr.
Timetrack Jr. is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
Timetrack Jr. is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
You should have received a copy of the GNU General Public License along with Timetrack Jr. If not, see <https://www.gnu.org/licenses/>.
*/
use crate::{
    cli,
    db::{self, TimeWindow},
    TTError,
};
use chrono::{DateTime, Datelike, Local, NaiveDateTime, Utc};
use icalendar::{Calendar, Component, Event};
use notify_rust::{Notification, Timeout};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct TimeWindowExport {
    pub id: Option<i64>,
    pub category: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub start_timestamp: String,
    pub end_timestamp: Option<String>,
}

impl From<TimeWindow> for TimeWindowExport {
    fn from(w: TimeWindow) -> Self {
        TimeWindowExport {
            id: w.id,
            category: w.category,
            start_time: w.start_time.clone(),
            end_time: w.end_time.clone(),
            start_timestamp: DateTime::<chrono::Local>::from(unix_to_utc(&w.start_time))
                .to_rfc3339(),
            end_timestamp: match w.end_time {
                Some(t) => Some(DateTime::<chrono::Local>::from(unix_to_utc(&t)).to_rfc3339()),
                None => None,
            },
        }
    }
}

fn export_json(
    outfile: &mut Box<dyn std::io::Write>,
    times: Vec<TimeWindow>,
) -> Result<(), TTError> {
    let times_export: Vec<TimeWindowExport> = times.into_iter().map(|t| t.into()).collect();
    outfile.write_all(serde_json::to_string_pretty(&times_export)?.as_bytes())?;
    Ok(())
}

fn unix_to_utc(tstamp: &i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(*tstamp, 0), Utc)
}

fn export_ical(
    outfile: &mut Box<dyn std::io::Write>,
    times: Vec<TimeWindow>,
) -> Result<(), TTError> {
    let mut calendar = Calendar::new();
    for time in times {
        if time.end_time.is_some() {
            calendar.push(
                Event::new()
                    .summary(&time.category)
                    .starts(unix_to_utc(&time.start_time))
                    .ends(unix_to_utc(&time.end_time.unwrap()))
                    .done(),
            );
        }
    }
    outfile.write_all(calendar.to_string().as_bytes())?;
    Ok(())
}
fn export_csv(
    outfile: &mut Box<dyn std::io::Write>,
    times: Vec<TimeWindow>,
) -> Result<(), TTError> {
    outfile.write_all(
        &"id,category,start,end,start_tstamp,end_tstamp,duration_hours,duration_seconds\n"
            .as_bytes(),
    )?;
    for time in times {
        outfile.write_all(
            &format!(
                "{},{},{},{},{},{},{},{}\n",
                time.id.unwrap_or(-1),
                time.category
                    .replace(",", ".")
                    .replace("\n", "")
                    .replace("\r", ""),
                DateTime::<chrono::Local>::from(unix_to_utc(&time.start_time)).to_rfc3339(),
                match time.end_time {
                    Some(end) => DateTime::<chrono::Local>::from(unix_to_utc(&end)).to_rfc3339(),
                    None => "".to_string(),
                },
                time.start_time,
                match time.end_time {
                    Some(end) => end.to_string(),
                    None => "".to_string(),
                },
                match time.end_time {
                    Some(end) => format!("{:.2}", ((end - time.start_time) as f64) / 60.0 / 60.0),
                    None => "".to_string(),
                },
                match time.end_time {
                    Some(end) => ((end - time.start_time) as f64).to_string(),
                    None => "".to_string(),
                },
            )
            .as_bytes(),
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct Summary {
    total: u64,
    count: u64,
}

fn export_summary(
    outfile: &mut Box<dyn std::io::Write>,
    times: Vec<TimeWindow>,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<(), TTError> {
    match (start, end) {
        (None, None) => outfile.write_all("Tabulating results for all time\n".as_bytes())?,
        (Some(s), None) => outfile.write_all(
            format!(
                "Tabulating results starting on/after {}\n",
                DateTime::<chrono::Local>::from(unix_to_utc(&s)).to_rfc2822()
            )
            .as_bytes(),
        )?,
        (None, Some(e)) => outfile.write_all(
            format!(
                "Tabulating results through {}\n",
                DateTime::<chrono::Local>::from(unix_to_utc(&e)).to_rfc2822()
            )
            .as_bytes(),
        )?,
        (Some(s), Some(e)) => outfile.write_all(
            format!(
                "Tabulating results starting on/after {} through {}\n",
                DateTime::<chrono::Local>::from(unix_to_utc(&s)).to_rfc2822(),
                DateTime::<chrono::Local>::from(unix_to_utc(&e)).to_rfc2822()
            )
            .as_bytes(),
        )?,
    }
    let mut category_totals = BTreeMap::<String, Summary>::new();
    for time in times {
        let summary = match category_totals.get_mut(&time.category) {
            Some(s) => s,
            None => {
                category_totals.insert(time.category.clone(), Summary { total: 0, count: 0 });
                category_totals.get_mut(&time.category).unwrap()
            }
        };
        summary.count += 1;
        if let Some(end) = time.end_time {
            summary.total += (unix_to_utc(&end) - unix_to_utc(&time.start_time))
                .num_seconds()
                .abs() as u64;
        }
    }
    if let Some((total_duration, total_count)) = category_totals
        .values()
        .map(|foo| (foo.total, foo.count))
        .reduce(|accum, item| (accum.0 + item.0, accum.1 + item.1))
    {
        outfile.write_all(
            format!(
                "Logged {} activites for a total of {:02}:{:02}\n",
                total_count,
                total_duration / 60 / 60,
                total_duration / 60 % 60
            )
            .as_bytes(),
        )?;

        for (category, summary) in category_totals {
            outfile.write_all(format!("{}:\n", category).as_bytes())?;
            outfile.write_all(
                format!(
                    "  {} logs, {:02}:{:02} cumulative, {:.2}% of total\n",
                    summary.count,
                    summary.total / 60 / 60,
                    summary.total / 60 % 60,
                    (summary.total as f64 / total_duration as f64) * 100 as f64
                )
                .as_bytes(),
            )?;
        }
    } else {
        return Err(TTError::TTError {
            message: "Didn't find any times to summarize".to_string(),
        });
    }
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
    let start = cli::time_string_to_tstamp(start_time);
    if start_time.is_some() && start.is_none() {
        return Err(TTError::TTError {
            message: "Was unable to parse start-time".to_string(),
        });
    }
    let end = cli::time_string_to_tstamp(end_time);
    if end_time.is_some() && end.is_none() {
        return Err(TTError::TTError {
            message: "was unable to parse end-time".to_string(),
        });
    }
    //fetch times from database
    let times = db::get_times(&mut tx, start, end)?;
    match format {
        cli::ExportFormat::Json => export_json(&mut handle, times)?,
        cli::ExportFormat::Csv => export_csv(&mut handle, times)?,
        cli::ExportFormat::Ical => export_ical(&mut handle, times)?,
        cli::ExportFormat::Summary => export_summary(&mut handle, times, start, end)?,
    }
    handle.flush()?;
    Ok(())
}

pub fn export(
    conn: &mut Connection,
    format: &cli::ExportFormat,
    listen: &bool,
    db_path: &String,
    outfile: &String,
    start_time: &Option<String>,
    end_time: &Option<String>,
) -> Result<(), TTError> {
    if *listen {
        let mut last_mod: Option<SystemTime> = None;
        loop {
            let current_mod = std::fs::metadata(db_path)?.modified()?;
            if last_mod.is_none() || last_mod.unwrap() != current_mod {
                match gen_export(conn, format, outfile, start_time, end_time) {
                    Err(e) => println!("Could not generate export! Error: {:?}", e),
                    _ => {}
                }
                last_mod = Some(current_mod);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    } else {
        return gen_export(conn, format, outfile, start_time, end_time);
    }
}

pub(crate) fn currently_timing(conn: &mut Connection, notify: &bool) -> Result<(), TTError> {
    let tx = conn.transaction()?;
    if let Some(open_time) = db::get_last_open_time(&tx)? {
        if *notify {
            let start_tstamp = unix_to_utc(&open_time.start_time);
            let duration_sec = (chrono::Utc::now() - start_tstamp).num_seconds();
            Notification::new()
                .appname("Timetrack Jr.")
                .summary(&format!("Currently timing \"{}\"", open_time.category))
                .body(&format!(
                    "Started: {}\nDuration: {:02}:{:02}:{:02}",
                    DateTime::<Local>::from(start_tstamp).to_rfc2822(),
                    duration_sec / 60 / 60,
                    duration_sec / 60 % 60,
                    duration_sec % 60,
                ))
                .show()?;
        }
        println!("{}", serde_json::to_string_pretty(&open_time)?)
    } else if *notify {
        Notification::new()
            .appname("Timetrack Jr.")
            .summary("Not currently timing")
            .timeout(Timeout::Milliseconds(5000))
            .show()?;
    }
    Ok(())
}
