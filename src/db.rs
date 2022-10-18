use rusqlite::{Connection};
use std::collections::{BTreeMap};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PRETTY_INDENT: u16 = 2;

pub struct Category {
    name: String,
    keys: Option<String>
}

pub struct TimeWindow {
    id: u64,
    category: String,
    start_time: u64,
    end_time: Option<u64>,
}

pub fn initialize_db(conn: &Connection) -> Result<&Connection, rusqlite::Error> {

    conn.execute("PRAGMA foreign_keys = ON", ())?;

    conn.execute("BEGIN", ());

    conn.execute(
        "CREATE TABLE IF NOT EXISTS options (
            name TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )", ()
    )?;

    //might use this later to handle DB migrations if that's a thing
    conn.execute(
        "REPLACE INTO options (name, value) VALUES ('dbversion', ?)",
        (VERSION,)
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            name TEXT PRIMARY KEY,
            keys TEXT NOT NULL
        )", ()
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS times (
            id INTEGER PRIMARY KEY,
            category TEXT NOT NULL,
            start_time INTEGER NOT NULL CHECK (start_time > 0),
            end_time INTEGER CHECK (end_time is null or end_time >= start_time),
            FOREIGN KEY(category) REFERENCES categories(name)
        )", ()
    )?;

    conn.execute("COMMIT", ())?;

    return Result::Ok(&conn);
}

pub fn print_config(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    let mut stmt = conn.prepare("SELECT name, value FROM options")?;
    let mut rows = stmt.query(())?;

    while let Some(row) = rows.next()? {
        options.insert(row.get(0)?, row.get(1)?);
    }

    let mut categories: Vec<String> = Vec::new();
    let mut stmt = conn.prepare("SELECT name FROM categories order by name")?;
    let mut rows = stmt.query(())?;

    while let Some(row) = rows.next()? {
        categories.push(row.get(0)?);
    }

    println!(
        "Options:\n{}\n==============\nCategories:\n{}", 
        json::stringify_pretty(options, PRETTY_INDENT),
        json::stringify_pretty(categories, PRETTY_INDENT)
    );

    return Ok(());
}