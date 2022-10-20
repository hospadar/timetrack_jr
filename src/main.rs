use clap::{Parser};
pub mod cli;
pub mod db;
pub mod commands;
pub mod keyboard;


fn main() {
    let cli = cli::Cli::parse();
    let conn = rusqlite::Connection::open(&cli.db_path.as_ref().unwrap()).expect("Couldn't open DB");

    db::initialize_db(&conn).expect("failed to initialize DB");

    commands::execute(&cli, &conn).unwrap();

    conn.close().expect("Failed to close DB")
}

