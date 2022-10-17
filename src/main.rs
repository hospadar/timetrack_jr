use rdev;
use clap::{Parser};
mod cli;
mod db;

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
    println!("{:?}", cli);
    let conn = rusqlite::Connection::open(cli.db_path.unwrap()).expect("Couldn't open DB");

    db::initialize_db(&conn).expect("failed to initialize DB");

    
}

