/*
This file is part of Timetrack Jr.
Timetrack Jr. is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
Timetrack Jr. is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
You should have received a copy of the GNU General Public License along with Timetrack Jr. If not, see <https://www.gnu.org/licenses/>.
*/
use chrono::{DateTime, Datelike};
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

fn roll_months<T: chrono::TimeZone>(date: &DateTime<T>, num_months: i32) -> DateTime<T> {
    let mut new_date = date.clone();
    if num_months == 0 {
        return new_date;
    }

    for _ in 0..num_months {
        if num_months < 0 {
            //decrement
            if new_date.month0() == 0 {
                //decrement the year
                new_date = new_date
                    .with_month0(11)
                    .unwrap()
                    .with_year(new_date.year() - 1)
                    .unwrap()
            }
            new_date = new_date.with_month0(new_date.month0() - 1).unwrap();
        } else {
            //increment
            if new_date.month0() == 11 {
                //increment the year
                new_date = new_date
                    .with_month0(0)
                    .unwrap()
                    .with_year(new_date.year() + 1)
                    .unwrap()
            }
            new_date = new_date.with_month0(new_date.month0() + 1).unwrap();
        }
    }

    return new_date;
}

pub fn time_string_to_tstamp(tstring: &Option<String>) -> Option<i64> {
    match tstring {
        Some(raw_time) => {
            if let Ok(parsed) = chrono_english::parse_date_string(
                raw_time,
                chrono::Local::now(),
                chrono_english::Dialect::Us,
            ) {
                Some(parsed.timestamp())
            } else if let Ok(parsed_duration) = chrono_english::parse_duration(raw_time) {
                let mut parsed_time = chrono::Local::now();
                match parsed_duration {
                    chrono_english::Interval::Seconds(n) => {
                        parsed_time += chrono::Duration::seconds(n as i64)
                    }
                    chrono_english::Interval::Days(n) => {
                        parsed_time += chrono::Duration::days(n as i64)
                    }
                    chrono_english::Interval::Months(n) => {
                        parsed_time = roll_months(&parsed_time, n)
                    }
                }
                return Some(parsed_time.timestamp());
            } else {
                None
            }
        }
        _ => None,
    }
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
    StartTiming {
        category_name: String,
        #[arg(short, long)]
        notify: bool,
    },
    ///End timing
    StopTiming {
        #[arg(short, long)]
        notify: bool,
    },
    AmendTime {
        time_id: i64,
        #[arg(short, long)]
        start_time: Option<String>,
        #[arg(short, long)]
        end_time: Option<String>,
        #[arg(short, long)]
        category: Option<String>,
    },
    ///Rename a category - updates any corresponding time as well
    RenameCategory {
        #[arg(short, long)]
        old: String,
        #[arg(short, long)]
        new: String,
    },
    ///If a time record is currently open, print out the category name.  Optionally send a desktop notification.  Handy to bind to a global shortcut to pop up a notification for what's being timed right now.
    CurrentlyTiming {
        #[arg(short, long)]
        notify: bool,
    },
    ///Delete any time records between a certain start and end time.
    BulkDeleteTimes {
        ///By default, delete any time whose start OR end are between --start-time and --end-time.  
        ///If --non-inclusive is set, do not delete a time unless its start AND end is between --start-time and --end-time
        #[arg(short, long)]
        non_inclusive: bool,
        #[arg(short, long)]
        start_time: String,
        #[arg(short, long)]
        end_time: String,
    },
    ///Delete a given time record.
    DeleteTime { time_id: i64 },
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
