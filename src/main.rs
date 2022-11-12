/*
Copyright 2022 Luke Hospadaruk
This file is part of Timetrack Jr.
Timetrack Jr. is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
Timetrack Jr. is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
You should have received a copy of the GNU General Public License along with Timetrack Jr. If not, see <https://www.gnu.org/licenses/>. 
*/
use clap::Parser;
use std::{
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

impl From<notify_rust::error::Error> for TTError {
    fn from(err: notify_rust::error::Error) -> Self {
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
