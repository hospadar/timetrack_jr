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
    ///Show config options and currently-registered-categories
    ShowConfig,
    ///Create a new category that you can use for time tracking
    AddCategory { category_name: String },
    ///Delete a category
    DeleteCategory {
        category_name: String,
        #[arg(short, long)]
        delete_logged_times: bool,
    },
    ///Set a global option
    SetOption {
        option_name: OptionName,
        option_value: String,
    },
    ///Remove an option
    UnsetOption { option_name: OptionName },
    ///Start timing an activity - stops timing any currently running activities
    StartTiming { category_name: String },
    ///End timing
    StopTiming,
    AmendTime {
        time_id: i64,
        #[arg(short, long)]
        start_time: Option<String>,
        #[arg(short, long)]
        end_time: Option<String>,
        #[arg(short, long)]
        category: Option<String>,
    },
    ///Export the DB to a more friendly format for analysis
    Export {
        ///Format of export to generate
        #[arg(short, long, value_enum)]
        format: ExportFormat,
        ///Watch underlying DB for changes and re-export any time a change happens
        #[arg(short, long)]
        listen: bool,
        ///Filename to export to - use `-` for stdout
        #[arg(short, long, default_value = "-")]
        outfile: String,
        ///Earliest entries to include in the extract (defaults to everything)
        #[arg(short, long)]
        start_time: Option<String>,
        ///Latest entries to include in the extract (defaults to everything)
        #[arg(short, long)]
        end_time: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportFormat {
    Json,
    Csv,
    Ical,
    Summary,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OptionName {
    EndOfDay,
}
