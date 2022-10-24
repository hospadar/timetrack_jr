use super::super::{db, keyboard};
use rusqlite::Connection;

pub fn show(conn: &Connection) -> Result<(), rusqlite::Error> {
    let config = db::get_config(conn)?;
    let json = match serde_json::to_string_pretty(&config) {
        Ok(j) => j,
        Err(error) => "Unable to serialize config: ".to_string() + error.to_string().as_str(),
    };

    println!("{}", json);

    Ok(())
}

pub fn add_category(
    conn: &Connection,
    category_name: &String,
    set_key_sequence: &bool,
) -> Result<(), rusqlite::Error> {
    let mut keypress = None;
    if *set_key_sequence {
        keypress = keyboard::get_keypress();
        match &keypress {
            Some(kp) => println!("Got key sequence: {}", kp),
            None => {
                println!("Didn't get a key sequence!");
                return Ok(());
            }
        }
    }
    db::add_category(conn, &category_name, keypress)?;
    Ok(())
}

pub fn delete_category(
    conn: &Connection,
    category_name: &String,
    delete_logged_times: &bool,
) -> Result<(), rusqlite::Error> {
    db::delete_category(conn, category_name, delete_logged_times)
}
