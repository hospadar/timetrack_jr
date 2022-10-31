use crate::TTError;
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{
    types::FromSql, Connection, Map, MappedRows, Row, Rows, Statement, ToSql, Transaction,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize)]
pub struct Config {
    options: Options,
    categories: Categories,
}

pub type Options = BTreeMap<String, String>;
pub type Categories = BTreeMap<String, Category>;

#[derive(Serialize, Deserialize)]
pub struct Category {
    name: String,
    keys: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct TimeWindow {
    id: Option<i64>,
    category: String,
    start_time: u64,
    end_time: Option<u64>,
}

static BUSINESS_HOURS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new("^(?P<hour>\\d{1,2}):(?P<minute>\\d{1,2})").unwrap());

#[derive(Eq, PartialEq, Debug)]
pub struct HourMinute(u8, u8);

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
    let mut stmt = conn.prepare("SELECT name, keys FROM categories order by name")?;
    let mut rows = stmt.query(())?;

    while let Some(row) = rows.next()? {
        categories.insert(
            row.get(0)?,
            Category {
                name: row.get(0)?,
                keys: row.get(1)?,
            },
        );
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
            .parse::<u8>()
            .unwrap();
        let minute = capture
            .name("minute")
            .unwrap()
            .as_str()
            .parse::<u8>()
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

pub fn get_open_times<'a>(tx: &'a Transaction) -> Result<Vec<TimeWindow>, TTError> {
    let mut stmt = tx.prepare("SELECT * FROM times WHERE end_time IS NULL")?;

    let results = rusqlite::Statement::query(&mut stmt, ())?;

    let mapped = results.mapped(|row| {
        Ok(TimeWindow {
            id: row.get("id")?,
            category: row.get("category")?,
            start_time: row.get("start_time")?,
            end_time: row.get("end_time")?,
        })
    });

    let mut allRows: Vec<TimeWindow> = vec![];
    for row in mapped {
        allRows.push(row?);
    }

    Ok(allRows)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;

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
        let mut conn = Connection::open_in_memory().unwrap();
        initialize_db(&mut conn).unwrap();
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
}
