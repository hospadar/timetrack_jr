use clap::{Parser, Subcommand};


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(long, default_value = "ttjr.sqlite3")]
    pub db_path: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    ///Set up DB and configure options
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands
    },
    ///Start or stop timing an activity, or amend an existing time
    Log {
        #[command(subcommand)]
        log_command: LogCommands
    },
    ///Globally listen for key events to trigger time tracking
    Listen {
        #[arg(long, short)]
        ical_file:Option<String>,
        #[arg(long, short)]
        csv_file:Option<String>, 
        #[arg(long, short)]
        json_file:Option<String>,
        #[arg(long, default_value = "-1")]
        days_to_export:i64
    },
}


#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    ///Show config options and currently-registered-categories
    Show,
    ///Create a new category that you can use for time tracking
    AddCategory {
        category_name: String,
        #[arg(short,long)]
        set_key_sequence: bool
    }, 
    ///Delete a category
    DeleteCategory {
        category_name: String,
        #[arg(short,long)]
        delete_logged_times: bool
    },
    ///Set the key sequence to trigger the start of timing when using listen mode
    SetKeySequence {
        category_name: String
    },
    ///Set a global option
    SetOption{
        option_name: String,
        option_value: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum LogCommands {
    StartTiming {
        category_name: String
    },
    StopTiming,
    AmendTime {
        time_id: u64,
        #[arg(short, long)]
        start_time: String,
        #[arg(short, long)]
        end_time: String,
        #[arg(short, long)]
        category: String
    }
}

