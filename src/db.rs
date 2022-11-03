use crate::{cli, TTError};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Timelike};
use clap::ValueEnum;
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{
    types::FromSql, Connection, Map, MappedRows, Row, Rows, Statement, ToSql, Transaction,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut},
    result,
    time::{Instant, SystemTime, UNIX_EPOCH},
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
        tx.execute("DELETE FROM times WHERE category", (&category_name,))?;
    }
    tx.execute("DELETE FROM categories WHERE name=?", (&category_name,))?;

    Ok(())
}

///Update a time in the DB.  does NOT commit the transaction
pub fn upsert_time(tx: &mut Transaction, time: TimeWindow) -> Result<(), TTError> {
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

        if (hour > 23) {
            return Err(TTError::TTError {
                message: format!("Got hour={}, but hour must be 0-23", hour),
            });
        } else if (minute > 59) {
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

#[cfg(test)]
mod tests {
    use std::{thread::Thread, time::Duration};

    use chrono::{NaiveDate, Offset};
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

            upsert_time(
                &mut tx,
                TimeWindow {
                    id: None,
                    category: "work".to_string(),
                    start_time: 51,
                    end_time: None,
                },
            )
            .unwrap();

            assert_eq!(
                Ok(TimeWindow {
                    id: Some(2),
                    category: "work".to_string(),
                    start_time: 51,
                    end_time: None
                }),
                get_time(&tx, tx.last_insert_rowid())
            );

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
