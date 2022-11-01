use std::time::{Duration, SystemTimeError};

use clap::Parser;
pub mod cli;
pub mod commands;
pub mod db;

pub type RusqliteError = rusqlite::Error;

#[derive(Debug, PartialEq)]
pub enum TTError {
    SqlError(rusqlite::Error),
    SystemTimeError(Duration),
    TTError { message: String },
}

impl From<rusqlite::Error> for TTError {
    fn from(err: rusqlite::Error) -> Self {
        TTError::SqlError(err)
    }
}

impl From<SystemTimeError> for TTError {
    fn from(err: SystemTimeError) -> Self {
        TTError::SystemTimeError(err.duration())
    }
}

fn main() {
    let cli = cli::Cli::parse();
    let mut conn =
        rusqlite::Connection::open(&cli.db_path.as_ref().unwrap()).expect("Couldn't open DB");

    db::initialize_db(&mut conn).expect("failed to initialize DB");

    commands::execute(&cli, &mut conn).unwrap();

    conn.close().expect("Unable to close DB cleanly");
}
