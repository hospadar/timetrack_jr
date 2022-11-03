use clap::Parser;
use std::{
    any::Any,
    num::ParseIntError,
    process::exit,
    time::{Duration, SystemTimeError},
};
pub mod cli;
pub mod commands;
pub mod db;

pub type RusqliteError = rusqlite::Error;

#[derive(Debug, PartialEq)]
pub enum TTError {
    SqlError(rusqlite::Error),
    SystemTimeError(Duration),
    ParseIntError(ParseIntError),
    TTError { message: String },
}

impl From<serde_json::Error> for TTError {
    fn from(err: serde_json::Error) -> Self {
        TTError::TTError {
            message: format!("{:?}", err),
        }
    }
}

impl From<ParseIntError> for TTError {
    fn from(err: ParseIntError) -> Self {
        TTError::ParseIntError(err)
    }
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

impl From<std::io::Error> for TTError {
    fn from(err: std::io::Error) -> Self {
        TTError::TTError {
            message: format!("{:?}", err),
        }
    }
}

fn main() {
    let cli = cli::Cli::parse();
    let mut conn =
        rusqlite::Connection::open(&cli.db_path.as_ref().unwrap()).expect("Couldn't open DB");

    db::initialize_db(&mut conn).expect("failed to initialize DB");

    let mut exit_code = 0;

    match commands::execute(&cli, &mut conn) {
        Err(TTError::TTError { message }) => {
            println!("{}", message);
            exit_code = 1;
        }
        Err(e) => {
            println!("Error!: {:?}", e);
            exit_code = 2;
        }
        _ => {}
    };

    conn.close().expect("Unable to close DB cleanly");

    exit(exit_code);
}
