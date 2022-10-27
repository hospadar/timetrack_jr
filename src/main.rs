use clap::Parser;
pub mod cli;
pub mod commands;
pub mod db;

pub type RusqliteError = rusqlite::Error;

#[derive(Debug, PartialEq)]
pub enum TTError {
    SqlError(rusqlite::Error),
    TTError { message: String },
}

impl From<rusqlite::Error> for TTError {
    fn from(err: rusqlite::Error) -> Self {
        TTError::SqlError(err)
    }
}

fn main() {
    let cli = cli::Cli::parse();
    let conn =
        rusqlite::Connection::open(&cli.db_path.as_ref().unwrap()).expect("Couldn't open DB");

    db::initialize_db(&conn).expect("failed to initialize DB");

    commands::execute(&cli, &conn).unwrap();

    conn.close().expect("Failed to close DB")
}
