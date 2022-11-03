use crate::{cli::OptionName, db, TTError};
use clap::ValueEnum;
use libsqlite3_sys;
use rusqlite::{Connection, ErrorCode};

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
    match db::delete_category(&tx, category_name, delete_logged_times) {
        Err(TTError::SqlError(rusqlite::Error::SqliteFailure(
            libsqlite3_sys::Error {
                code: libsqlite3_sys::ErrorCode::ConstraintViolation,
                extended_code: _,
            },
            _,
        ))) => {
            return Err(TTError::TTError { message: "Unable to delete category because times have been logged with that category.  Add --delete-logged-times to delete the category AND any times logged with the category".to_string()});
        }
        Err(e) => {
            return Err(e);
        }
        Ok(_) => {}
    }
    tx.commit()?;
    Ok(())
}

pub fn set_option(
    conn: &mut Connection,
    option_name: &OptionName,
    option_value: &String,
) -> Result<(), TTError> {
    //validate option values if necessary
    match option_name {
        OptionName::EndOfDay => {
            //check that end of day has correct format
            db::parse_time(option_value)?;
        }
    }
    let tx = conn.transaction()?;
    db::set_option(&tx, option_name, option_value)?;
    tx.commit()?;
    Ok(())
}

pub fn unset_option(conn: &mut Connection, option_name: &OptionName) -> Result<(), TTError> {
    let tx = conn.transaction()?;
    db::unset_option(&tx, option_name)?;
    tx.commit()?;
    Ok(())
}
