use rusqlite::{Connection};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
            keys TEXT NOT NULL,
            press_and_hold BOOLEAN NOT NULL
        )", ()
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS times (
            id INTEGER PRIMARY KEY,
            category TEXT NOT NULL,
            start_time INTEGER NOT NULL,
            end_time INTEGER NOT NULL CHECK (end_time >= start_time),
            FOREIGN KEY(category) REFERENCES categories(name)
        )", ()
    )?;

    conn.execute("COMMIT", ())?;

    return Result::Ok(&conn);
}