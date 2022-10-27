use crate::TTError;

use rusqlite::{Connection, ToSql, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{BTreeMap, HashSet},
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

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;

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
    }
}
