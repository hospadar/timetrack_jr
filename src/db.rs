/*
This file is part of Timetrack Jr.
Timetrack Jr. is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
Timetrack Jr. is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
You should have received a copy of the GNU General Public License along with Timetrack Jr. If not, see <https://www.gnu.org/licenses/>.
*/

use crate::{cli, TTError};
use chrono::{DateTime, NaiveDateTime, Timelike};
use clap::ValueEnum;
use fallible_iterator::FallibleIterator;
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{named_params, Connection, Row, ToSql, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize)]
pub struct Config {
    options: Options,
    categories: Categories,
}

pub type Options = BTreeMap<String, String>;
pub type Categories = BTreeSet<String>;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct TimeWindow {
    pub id: Option<i64>,
    pub category: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
}

fn row_to_time_window(row: &Row) -> Result<TimeWindow, rusqlite::Error> {
    Ok(TimeWindow {
        id: row.get("id")?,
        category: row.get("category")?,
        start_time: row.get("start_time")?,
        end_time: row.get("end_time")?,
    })
}

static BUSINESS_HOURS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new("^(?P<hour>\\d{1,2}):(?P<minute>\\d{1,2})").unwrap());

#[derive(Eq, PartialEq, Debug)]
pub struct HourMinute(u32, u32);

impl std::fmt::Display for HourMinute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.0, self.1)
    }
}

impl std::cmp::Ord for HourMinute {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl std::cmp::PartialOrd for HourMinute {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_string().partial_cmp(&other.to_string())
    }
}

pub fn initialize_db(conn: &mut Connection) -> Result<(), TTError> {
    conn.execute("PRAGMA foreign_keys = ON", ())?;

    let tx = conn.transaction()?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS options (
            name TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        (),
    )?;

    //might use this later to handle DB migrations if that's a thing
    tx.execute(
        "REPLACE INTO options (name, value) VALUES ('dbversion', ?)",
        (VERSION,),
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            name TEXT PRIMARY KEY
        )",
        (),
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS times (
            id INTEGER PRIMARY KEY,
            category TEXT NOT NULL,
            start_time INTEGER NOT NULL CHECK (start_time >= 0),
            end_time INTEGER CHECK (end_time is null or end_time >= start_time),
            FOREIGN KEY(category) REFERENCES categories(name) ON UPDATE CASCADE ON DELETE RESTRICT
        )",
        (),
    )?;

    tx.commit()?;

    return Ok(());
}

pub fn set_option(
    tx: &Transaction,
    option_name: &cli::OptionName,
    option_value: &String,
) -> Result<(), TTError> {
    if let Some(option_name) = ValueEnum::to_possible_value(option_name) {
        tx.execute(
            "REPLACE INTO options (name, value) VALUES (?, ?)",
            (option_name.get_name(), option_value),
        )?;
        Ok(())
    } else {
        Err(TTError::TTError {
            message: format!("Unknown Option Name {:?}", option_name),
        })
    }
}

pub fn unset_option(tx: &Transaction, option_name: &cli::OptionName) -> Result<(), TTError> {
    if let Some(option_name) = ValueEnum::to_possible_value(option_name) {
        tx.execute(
            "DELETE FROM options WHERE name = ?",
            (option_name.get_name(),),
        )?;
        Ok(())
    } else {
        Err(TTError::TTError {
            message: format!("Unknown Option Name {:?}", option_name),
        })
    }
}

pub fn get_options(conn: &Transaction) -> Result<Options, TTError> {
    let mut options: Options = Options::new();
    let mut stmt = conn.prepare("SELECT name, value FROM options")?;
    let mut rows = stmt.query(())?;

    while let Some(row) = rows.next()? {
        options.insert(row.get(0)?, row.get(1)?);
    }

    Ok(options)
}

pub fn get_categories(conn: &Transaction) -> Result<Categories, TTError> {
    let mut categories = Categories::new();
    let mut stmt = conn.prepare("SELECT name FROM categories order by name")?;
    let mut rows = stmt.query(())?;

    while let Some(row) = rows.next()? {
        categories.insert(row.get(0)?);
    }

    Ok(categories)
}

pub fn get_config(conn: &Transaction) -> Result<Config, TTError> {
    return Ok(Config {
        options: get_options(conn)?,
        categories: get_categories(conn)?,
    });
}

pub fn add_category(conn: &Transaction, category_name: &String) -> Result<(), TTError> {
    conn.execute("INSERT INTO categories (name) VALUES (?)", (category_name,))?;
    Ok(())
}

pub fn delete_category(
    tx: &Transaction,
    category_name: &String,
    delete_logged_times: &bool,
) -> Result<(), TTError> {
    if *delete_logged_times {
        tx.execute("DELETE FROM times WHERE category=?", (&category_name,))?;
    }
    tx.execute("DELETE FROM categories WHERE name=?", (&category_name,))?;

    Ok(())
}

///Update a time in the DB.  does NOT commit the transaction
pub fn upsert_time(tx: &mut Transaction, time: TimeWindow) -> Result<(), TTError> {
    //must not overlap with an existing complete time
    //if there is an on open time, the time being upserted must be:
    //  a.the same time
    //  b. a different time AND not overlapping with the _start_ of the open time

    //disallow overlapping time entries
    let mut stmt = tx.prepare(
        "SELECT id c \
        FROM times
        WHERE 
            (id IS DISTINCT FROM :id) 
            AND (
                --upserted start time is in the middle of an already-recorded time
                (:start >= start_time AND  :start <= end_time)
                
                --upserted end time is in the middle of an already-recorded time
                --use coalesce because :end might be null
                OR COALESCE(:end >= start_time AND :end <= end_time, FALSE)

                --If there is an open time, the upserted time must be entirely before the open time
                OR (end_time IS NULL AND (:start >= start_time OR COALESCE(:end >= start_time, FALSE)))
            )
        ")?;
    let rows = stmt.query(named_params! {
        ":id": time.id,
        ":start": time.start_time,
        ":end": time.end_time
    })?;
    let overlapping_ids: Vec<String> = rows
        .map(|row| -> Result<i64, _> { row.get(0) })
        .collect::<Vec<i64>>()?
        .iter()
        .map(|i| i.to_string())
        .collect();
    if overlapping_ids.len() > 0 {
        return Err(TTError::TTError {
            message: format!(
                "Attempted to insert time that overlaps with other times! (overlapped IDs: {}) (time to insert: {:?}) (example overlap: {:?})",
                overlapping_ids.join(", "),
                time,
                get_time(tx, str::parse::<i64>(overlapping_ids.get(0).unwrap()).unwrap()).unwrap()
            ),
        });
    }

    let mut params: Vec<(&str, &dyn ToSql)> = Vec::new();

    if let Some(id) = &time.id {
        params.push((":id", id));
    }
    if let Some(end_time) = &time.end_time {
        params.push((":end_time", end_time));
    }

    params.push((":category", &time.category));

    params.push((":start_time", &time.start_time));

    let param_names: Vec<String> = params
        .iter()
        .map(|(name, _)| name[1..].to_string())
        .collect();

    let param_placeholders: Vec<&str> = params.iter().map(|(name, _)| *name).collect();

    let query = format!(
        "REPLACE INTO times ({}) VALUES ({})",
        param_names.join(", "),
        param_placeholders.join(", ")
    );

    tx.execute(&query[..], &params[..])?;

    Ok(())
}

pub fn get_time(tx: &Transaction, id: i64) -> Result<TimeWindow, TTError> {
    tx.query_row_and_then("SELECT * FROM times WHERE id=?", (id,), |row| {
        Ok(TimeWindow {
            id: Some(row.get("id").unwrap()),
            category: row.get("category").unwrap(),
            start_time: row.get("start_time").unwrap(),
            end_time: row.get("end_time").unwrap(),
        })
    })
}

pub fn get_last_open_time(tx: &Transaction) -> Result<Option<TimeWindow>, TTError> {
    let mut stmt =
        tx.prepare("SELECT * FROM times WHERE end_time IS NULL ORDER BY start_time DESC LIMIT 1")?;
    let mut rows = stmt.query(())?;
    if let Some(row) = rows.next()? {
        Ok(Some(TimeWindow {
            id: Some(row.get("id").unwrap()),
            category: row.get("category").unwrap(),
            start_time: row.get("start_time").unwrap(),
            end_time: row.get("end_time").unwrap(),
        }))
    } else {
        Ok(None)
    }
}

///given an HH:MM string, parses and validates to make sure it looks like a valid
/// 24-hour time and then returns a tuple of the parsed values
pub fn parse_time(time_string: &String) -> Result<HourMinute, TTError> {
    if let Some(capture) = BUSINESS_HOURS_PATTERN.captures(time_string) {
        let hour = capture
            .name("hour")
            .unwrap()
            .as_str()
            .parse::<u32>()
            .unwrap();
        let minute = capture
            .name("minute")
            .unwrap()
            .as_str()
            .parse::<u32>()
            .unwrap();

        if hour > 23 {
            return Err(TTError::TTError {
                message: format!("Got hour={}, but hour must be 0-23", hour),
            });
        } else if minute > 59 {
            return Err(TTError::TTError {
                message: format!("Got minute={}, but minute must be 0-59", minute),
            });
        } else {
            return Ok(HourMinute(hour, minute));
        }
    } else {
        return Err(TTError::TTError {
            message: "Time must a 24-hour time formatted like HH:MM (i.e. 10:30, 09:15, 8:00, etc)"
                .to_string(),
        });
    }
}

///End any times which don't have a recorded end time.
/// End times are set to the lesser of <current time> <next EOB (relative to start time)>
pub fn end_open_times(tx: &mut Transaction, end_of_business: HourMinute) -> Result<(), TTError> {
    let mut updated_times: Vec<TimeWindow> = vec![];
    {
        let mut stmt = tx.prepare("SELECT * FROM times WHERE end_time IS NULL")?;

        let mut results = stmt.query(())?;

        while let Some(row) = results.next()? {
            let mut logged_time = row_to_time_window(row)?;
            let start_date: DateTime<chrono::Local> = DateTime::from_utc(
                NaiveDateTime::from_timestamp(logged_time.start_time, 0),
                *chrono::Local::now().offset(),
            );

            //calculate the first EOB datetime that is AFTER the logged start time
            //set the hour and minute to EOB
            let mut end_date = start_date
                .clone()
                .with_hour(end_of_business.0)
                .unwrap()
                .with_minute(end_of_business.1)
                .unwrap();
            //if the end_date is before the start date (i.e. if the hour/minute of the start time is AFTER EOB)
            //then bump out the date by one day for the end time
            if end_date <= start_date {
                end_date += chrono::Duration::days(1);
            }

            let now_date = chrono::Local::now();

            logged_time.end_time = Some(std::cmp::min(end_date, now_date).timestamp());

            updated_times.push(logged_time);
        }
    }

    for time in updated_times {
        upsert_time(tx, time)?;
    }

    Ok(())
}

pub fn end_open_times_immediately(tx: &mut Transaction) -> Result<(), TTError> {
    tx.execute(
        "UPDATE times SET end_time = ? WHERE end_time is null ",
        (SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),),
    )?;

    return Ok(());
}

pub fn start_timing(tx: &mut Transaction, category: &String) -> Result<(), TTError> {
    upsert_time(
        tx,
        TimeWindow {
            id: None,
            category: category.clone(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
            end_time: None,
        },
    )
}

pub fn delete_time(tx: &mut Transaction, id: &i64) -> Result<usize, TTError> {
    Ok(tx.execute("DELETE FROM times WHERE id=?", (id,))?)
}

pub fn get_times(
    tx: &mut Transaction,
    start_date: Option<i64>,
    end_date: Option<i64>,
) -> Result<Vec<TimeWindow>, TTError> {
    let mut clauses = Vec::<&str>::new();
    let mut values: Vec<&dyn ToSql> = vec![];
    let mut where_clause = String::new();
    if let Some(start) = &start_date {
        clauses.push("start_time >= ?");
        values.push(start);
    }
    if let Some(end) = &end_date {
        clauses.push("start_time <= ?");
        values.push(end);
    }

    if values.len() > 0 {
        where_clause = format!("WHERE {}", clauses.join(" AND "));
    }

    let mut stmt = tx.prepare(&format!(
        "SELECT id, category, start_time, end_time FROM times {}",
        where_clause
    ))?;

    for i in 1..(values.len() + 1) {
        stmt.raw_bind_parameter(i, values.get(i - 1).unwrap())?;
    }
    let rows = stmt.raw_query().mapped(|row| row_to_time_window(row));
    let mut times: Vec<TimeWindow> = Vec::new();

    for row in rows {
        times.push(row?)
    }

    return Ok(times);
}

pub fn rename_category(tx: &mut Transaction, old: &String, new: &String) -> Result<(), TTError> {
    let categories = get_categories(tx)?;

    if !categories.contains(old) {
        return Err(TTError::TTError {
            message: format!(
                "Category \"{0}\" cannot be renamed to \"{1}\" because \"{0}\" does not exist",
                old, new
            ),
        });
    }

    let mut stmt = tx.prepare("UPDATE categories SET name=? WHERE name=?")?;
    stmt.execute((new, old))?;

    //let mut stmt = tx.prepare("ALTER TABLE times SET category=? WHERE category=?")?;
    //stmt.execute((new, old))?;

    Ok(())
}

pub fn bulk_delete_times(
    tx: &mut Transaction,
    start_time: &i64,
    end_time: &i64,
    non_inclusive: &bool,
) -> Result<usize, TTError> {
    if !(end_time > start_time) {
        return Err(TTError::TTError {
            message: format!(
                "end time ({}) must be greater than start time ({})",
                end_time, start_time
            ),
        });
    }
    let mut stmt = tx.prepare("
        DELETE FROM times 
        WHERE CASE WHEN :non_inclusive 
            --non-inclusive case - only times which are completely inside the window
            THEN (start_time >= :start AND end_time <= :end) 
            -- default case - any time whose start or end is inside the window
            ELSE (start_time >= :start AND start_time <= :end) OR (end_time >= :start AND end_time <= :end) 
            END")?;
    let rows_deleted = stmt.execute(named_params! {
        ":non_inclusive": non_inclusive,
        ":start": start_time,
        ":end": end_time
    })?;
    Ok(rows_deleted)
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use chrono::NaiveDate;
    use rusqlite::Connection;

    use super::*;

    fn get_initialized_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize_db(&mut conn).unwrap();
        return conn;
    }

    #[test]
    fn test_timing() {
        let mut conn = get_initialized_db();
        {
            let mut tx = conn.transaction().unwrap();
            assert!(start_timing(&mut tx, &"work".to_string()).is_err());

            add_category(&mut tx, &"work".to_string()).unwrap();

            assert!(start_timing(&mut tx, &"work".to_string()).is_ok());
            let mut time = get_time(&tx, 1).unwrap();
            assert_eq!(Some(1), time.id);
            assert_eq!("work".to_string(), time.category);
            assert_eq!(None, time.end_time);

            std::thread::sleep(Duration::from_secs(1));

            end_open_times_immediately(&mut tx).unwrap();
            time = get_time(&tx, 1).unwrap();
            assert_eq!(Some(1), time.id);
            assert_eq!("work".to_string(), time.category);
            assert!(time.end_time.is_some());
            assert!(time.end_time.unwrap() > time.start_time);

            //un-set the end time
            let start_datetime = DateTime::<chrono::Local>::from_local(
                NaiveDate::from_ymd(2020, 12, 31).and_hms(12, 12, 0),
                *chrono::Local::now().offset(),
            );
            time.end_time = None;
            time.start_time = start_datetime.timestamp();
            upsert_time(&mut tx, time).unwrap();

            end_open_times(&mut tx, HourMinute(13, 0)).unwrap();

            time = get_time(&tx, 1).unwrap();

            //should have been ended at EOB
            assert_eq!(
                time.end_time.unwrap(),
                start_datetime
                    .with_hour(13)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .timestamp()
            );

            //What if EOB is less than start time
            time = get_time(&tx, 1).unwrap();
            time.end_time = None;
            upsert_time(&mut tx, time).unwrap();
            end_open_times(&mut tx, HourMinute(11, 0)).unwrap();
            time = get_time(&tx, 1).unwrap();
            //should have been ended at EOB the next day
            assert_eq!(
                time.end_time.unwrap(),
                (start_datetime
                    .with_hour(11)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    + chrono::Duration::days(1))
                .timestamp()
            );

            //what if current time is less than next EOB?
            //What if EOB is less than start time
            time = get_time(&tx, 1).unwrap();
            time.end_time = None;
            let start_datetime = chrono::Local::now();
            let mut eob = HourMinute(0, 0);
            if start_datetime.hour() == 0 {
                eob.0 = 23
            }
            time.start_time = start_datetime.timestamp();
            upsert_time(&mut tx, time).unwrap();
            end_open_times(&mut tx, eob).unwrap();
            time = get_time(&tx, 1).unwrap();
            //should have been ended nowish not EOB
            assert!(start_datetime.timestamp() - time.end_time.unwrap() < 10,);
        }
        conn.close().unwrap();
    }

    #[test]
    fn test_hour_minute_format() {
        assert_eq!("01:01", HourMinute(1, 1).to_string());
        assert_eq!("12:01", HourMinute(12, 1).to_string());
        assert_eq!("12:12", HourMinute(12, 12).to_string());
    }

    #[test]
    fn test_re() {
        assert!(BUSINESS_HOURS_PATTERN.is_match("00:11"));
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("01:02")
                .unwrap()
                .name("hour")
                .unwrap()
                .as_str(),
            "01"
        );
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("01:02")
                .unwrap()
                .name("minute")
                .unwrap()
                .as_str(),
            "02"
        );
        assert!(BUSINESS_HOURS_PATTERN.is_match("1:01"));
        assert!(BUSINESS_HOURS_PATTERN.is_match("1:2"));
        assert!(BUSINESS_HOURS_PATTERN.is_match("01:2"));
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("1:02")
                .unwrap()
                .name("hour")
                .unwrap()
                .as_str(),
            "1"
        );
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("01:2")
                .unwrap()
                .name("minute")
                .unwrap()
                .as_str(),
            "2"
        );
        assert!(!BUSINESS_HOURS_PATTERN.is_match(""));
        assert!(!BUSINESS_HOURS_PATTERN.is_match("1:"));
        assert!(!BUSINESS_HOURS_PATTERN.is_match(":1"));
        assert!(!BUSINESS_HOURS_PATTERN.is_match("a1:2"));
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(HourMinute(0, 0), parse_time(&"0:0".to_string()).unwrap());
        assert_eq!(HourMinute(1, 2), parse_time(&"01:02".to_string()).unwrap());
        assert_eq!(
            HourMinute(23, 59),
            parse_time(&"23:59".to_string()).unwrap()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got hour=99, but hour must be 0-23".to_string()
            },
            parse_time(&"99:0".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got minute=99, but minute must be 0-59".to_string()
            },
            parse_time(&"0:99".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got hour=99, but hour must be 0-23".to_string()
            },
            parse_time(&"99:99".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got hour=24, but hour must be 0-23".to_string()
            },
            parse_time(&"24:0".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got minute=60, but minute must be 0-59".to_string()
            },
            parse_time(&"23:60".to_string()).unwrap_err()
        );
    }

    #[test]
    pub fn test_rename_category() {
        let mut conn = get_initialized_db();
        {
            let mut tx = conn.transaction().unwrap();
            add_category(&tx, &"work".to_string()).unwrap();

            let mut expected: BTreeSet<String> = BTreeSet::new();
            expected.insert("work".to_string());
            assert_eq!(expected, get_categories(&tx).unwrap());

            upsert_time(
                &mut tx,
                TimeWindow {
                    id: None,
                    category: "work".to_string(),
                    start_time: 47,
                    end_time: None,
                },
            )
            .unwrap();

            assert_eq!(
                Ok(TimeWindow {
                    id: Some(1),
                    category: "work".to_string(),
                    start_time: 47,
                    end_time: None
                }),
                get_time(&tx, tx.last_insert_rowid())
            );

            //now rename the category, it should rename any times as well
            rename_category(&mut tx, &"work".to_string(), &"play".to_string()).unwrap();
            let mut expected: BTreeSet<String> = BTreeSet::new();
            expected.insert("play".to_string());
            assert_eq!(expected, get_categories(&tx).unwrap());

            assert_eq!(
                Ok(TimeWindow {
                    id: Some(1),
                    category: "play".to_string(),
                    start_time: 47,
                    end_time: None
                }),
                get_time(&tx, tx.last_insert_rowid())
            );
        }
    }

    #[test]
    pub fn test_upsert() {
        let mut conn = get_initialized_db();
        {
            let mut tx = conn.transaction().unwrap();
            add_category(&tx, &"work".to_string()).unwrap();
            upsert_time(
                &mut tx,
                TimeWindow {
                    id: None,
                    category: "work".to_string(),
                    start_time: 47,
                    end_time: None,
                },
            )
            .unwrap();

            assert_eq!(
                Ok(TimeWindow {
                    id: Some(1),
                    category: "work".to_string(),
                    start_time: 47,
                    end_time: None
                }),
                get_time(&tx, tx.last_insert_rowid())
            );

            //should fail because it potentially overlaps an open time
            assert_matches!(
                upsert_time(
                    &mut tx,
                    TimeWindow {
                        id: None,
                        category: "work".to_string(),
                        start_time: 51,
                        end_time: None,
                    },
                ),
                Err(_)
            );

            //should fail because it potentially overlaps an open time
            assert_matches!(
                upsert_time(
                    &mut tx,
                    TimeWindow {
                        id: None,
                        category: "work".to_string(),
                        start_time: 40,
                        end_time: Some(51),
                    },
                ),
                Err(_)
            );

            //close off the open time
            upsert_time(
                &mut tx,
                TimeWindow {
                    id: Some(1),
                    category: "work".to_string(),
                    start_time: 47,
                    end_time: Some(51),
                },
            )
            .unwrap();

            //now should be able to insert a new open time
            upsert_time(
                &mut tx,
                TimeWindow {
                    id: Some(2),
                    category: "work".to_string(),
                    start_time: 52,
                    end_time: None,
                },
            )
            .unwrap();

            //check that new open time was inserted correctly
            assert_eq!(
                Ok(TimeWindow {
                    id: Some(2),
                    category: "work".to_string(),
                    start_time: 52,
                    end_time: None
                }),
                get_time(&tx, tx.last_insert_rowid())
            );

            //should fail because it potentially overlaps an open time
            assert_matches!(
                upsert_time(
                    &mut tx,
                    TimeWindow {
                        id: None,
                        category: "work".to_string(),
                        start_time: 48,
                        end_time: None,
                    },
                ),
                Err(_)
            );

            //should fail because it overlaps closed time
            assert_matches!(
                upsert_time(
                    &mut tx,
                    TimeWindow {
                        id: None,
                        category: "work".to_string(),
                        start_time: 40,
                        end_time: Some(48),
                    },
                ),
                Err(_)
            );

            //change the start and end time
            upsert_time(
                &mut tx,
                TimeWindow {
                    id: Some(2),
                    category: "work".to_string(),
                    start_time: 111,
                    end_time: Some(112),
                },
            )
            .unwrap();
            //check that modified time is reflected in DB
            assert_eq!(
                Ok(TimeWindow {
                    id: Some(2),
                    category: "work".to_string(),
                    start_time: 111,
                    end_time: Some(112)
                }),
                get_time(&tx, tx.last_insert_rowid())
            );
        }
        conn.close().unwrap();
    }
}
