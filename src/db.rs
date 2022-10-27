use crate::TTError;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

#[derive(Serialize, Deserialize)]
pub struct TimeWindow {
    id: u64,
    category: String,
    start_time: u64,
    end_time: Option<u64>,
}

pub fn initialize_db(conn: &Connection) -> Result<&Connection, TTError> {
    conn.execute("PRAGMA foreign_keys = ON", ())?;

    conn.execute("BEGIN", ())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS options (
            name TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        (),
    )?;

    //might use this later to handle DB migrations if that's a thing
    conn.execute(
        "REPLACE INTO options (name, value) VALUES ('dbversion', ?)",
        (VERSION,),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            name TEXT PRIMARY KEY,
            keys TEXT NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS times (
            id INTEGER PRIMARY KEY,
            category TEXT NOT NULL,
            start_time INTEGER NOT NULL CHECK (start_time > 0),
            end_time INTEGER CHECK (end_time is null or end_time >= start_time),
            FOREIGN KEY(category) REFERENCES categories(name) ON UPDATE CASCADE ON DELETE RESTRICT
        )",
        (),
    )?;

    conn.execute("COMMIT", ())?;

    return Result::Ok(&conn);
}

pub fn get_options(conn: &Connection) -> Result<Options, TTError> {
    let mut options: Options = Options::new();
    let mut stmt = conn.prepare("SELECT name, value FROM options")?;
    let mut rows = stmt.query(())?;

    while let Some(row) = rows.next()? {
        options.insert(row.get(0)?, row.get(1)?);
    }

    Ok(options)
}

pub fn get_categories(conn: &Connection) -> Result<Categories, TTError> {
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

pub fn get_config(conn: &Connection) -> Result<Config, TTError> {
    return Ok(Config {
        options: get_options(conn)?,
        categories: get_categories(conn)?,
    });
}

pub fn add_category(conn: &Connection, category_name: &String) -> Result<(), TTError> {
    conn.execute("INSERT INTO categories (name) VALUES (?)", (category_name,))?;
    Ok(())
}

pub fn delete_category(
    conn: &Connection,
    category_name: &String,
    delete_logged_times: &bool,
) -> Result<(), TTError> {
    conn.execute("BEGIN", ())?;
    if *delete_logged_times {
        conn.execute("DELETE FROM times WHERE category", (&category_name,))?;
    }
    conn.execute("DELETE FROM categories WHERE name=?", (&category_name,))?;

    Ok(())
}
