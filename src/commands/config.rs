use rusqlite::{Connection};
use super::super::db;

pub fn show(conn: &Connection) -> Result<(), rusqlite::Error> {

    let config = db::get_config(conn)?;
    let json = match serde_json::to_string_pretty(&config) {
        Ok(j) => j,
        Err(error) => "Unable to serialize config: ".to_string() + error.to_string().as_str()
    };

    println!("{}", json);

    Ok(())
}