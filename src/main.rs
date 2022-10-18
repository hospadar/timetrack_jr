use rdev;
use clap::{Parser};
pub mod cli;
pub mod db;
pub mod commands;

fn callback(event: rdev::Event) {
    println!("My callback {:?}", event);
    match event.name {
        Some(string) => println!("User wrote {:?}", string),
        None => (),
    }
}

fn listen() {
    if let Err(error) = rdev::listen(callback) {
        println!("Error: {:?}", error)
    }
}

fn main() {
    let cli = cli::Cli::parse();
    let conn = rusqlite::Connection::open(&cli.db_path.as_ref().unwrap()).expect("Couldn't open DB");

    db::initialize_db(&conn).expect("failed to initialize DB");

    commands::execute(&cli, &conn).unwrap();
    
}

