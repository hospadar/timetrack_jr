use rdev;
use clap::{Parser};
mod cli;

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
    
}

