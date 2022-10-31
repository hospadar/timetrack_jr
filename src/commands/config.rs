use crate::{cli::OptionName, db, TTError};
use rusqlite::Connection;

pub fn show(conn: &mut Connection) -> Result<(), TTError> {
    let tx = conn.transaction()?;
    let config = db::get_config(&tx)?;
    let json = match serde_json::to_string_pretty(&config) {
        Ok(j) => j,
        Err(error) => "Unable to serialize config: ".to_string() + error.to_string().as_str(),
    };
    println!("{}", json);
    Ok(())
}

pub fn add_category(conn: &mut Connection, category_name: &String) -> Result<(), TTError> {
    let tx = conn.transaction()?;
    db::add_category(&tx, &category_name)?;
    tx.commit()?;
    Ok(())
}

pub fn delete_category(
    conn: &mut Connection,
    category_name: &String,
    delete_logged_times: &bool,
) -> Result<(), TTError> {
    let tx = conn.transaction()?;
    db::delete_category(&tx, category_name, delete_logged_times)?;
    tx.commit()?;
    Ok(())
}

pub fn set_option(
    conn: &mut Connection,
    option_name: &OptionName,
    option_value: &String,
) -> Result<(), TTError> {
    match option_name {
        OptionName::StartOfDay => todo!(),
        OptionName::EndOfDay => todo!(),
        OptionName::DaysOfWeek => todo!(),
    }
}
