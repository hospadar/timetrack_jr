use crate::{cli::OptionName, db, TTError};
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::Connection;

pub fn show(conn: &Connection) -> Result<(), TTError> {
    let config = db::get_config(conn)?;
    let json = match serde_json::to_string_pretty(&config) {
        Ok(j) => j,
        Err(error) => "Unable to serialize config: ".to_string() + error.to_string().as_str(),
    };

    println!("{}", json);

    Ok(())
}

pub fn add_category(conn: &Connection, category_name: &String) -> Result<(), TTError> {
    db::add_category(conn, &category_name)?;
    Ok(())
}

pub fn delete_category(
    conn: &Connection,
    category_name: &String,
    delete_logged_times: &bool,
) -> Result<(), TTError> {
    db::delete_category(conn, category_name, delete_logged_times)
}

pub fn set_option(
    conn: &Connection,
    option_name: &OptionName,
    option_value: &String,
) -> Result<(), TTError> {
    match option_name {
        OptionName::StartOfDay => todo!(),
        OptionName::EndOfDay => todo!(),
        OptionName::DaysOfWeek => todo!(),
    }
}

static BUSINESS_HOURS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new("^(?P<hour>\\d{1,2}):(?P<minute>\\d{1,2})").unwrap());

#[derive(Eq, PartialEq, Debug)]
pub struct HourMinute(u8, u8);

///given an HH:MM string, parses and validates to make sure it looks like a valid
/// 24-hour time and then returns a tuple of the parsed values
fn parse_time(time_string: &String) -> Result<HourMinute, TTError> {
    if let Some(capture) = BUSINESS_HOURS_PATTERN.captures(time_string) {
        let hour = capture
            .name("hour")
            .unwrap()
            .as_str()
            .parse::<u8>()
            .unwrap();
        let minute = capture
            .name("minute")
            .unwrap()
            .as_str()
            .parse::<u8>()
            .unwrap();

        if (hour > 23) {
            return Err(TTError::TTError {
                message: format!("Got hour={}, but hour must be 0-23", hour),
            });
        } else if (minute > 59) {
            return Err(TTError::TTError {
                message: format!("Got minute={}, but minute must be 0-59", minute),
            });
        } else {
            return Ok(HourMinute(hour, minute));
        }
    } else {
        return Err(TTError::TTError {
            message: "Time must a 24-hour time formatted like HH:MM (i.e. 10:30, 09:15, 8:00, etc)"
                .to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_re() {
        assert!(BUSINESS_HOURS_PATTERN.is_match("00:11"));
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("01:02")
                .unwrap()
                .name("hour")
                .unwrap()
                .as_str(),
            "01"
        );
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("01:02")
                .unwrap()
                .name("minute")
                .unwrap()
                .as_str(),
            "02"
        );
        assert!(BUSINESS_HOURS_PATTERN.is_match("1:01"));
        assert!(BUSINESS_HOURS_PATTERN.is_match("1:2"));
        assert!(BUSINESS_HOURS_PATTERN.is_match("01:2"));
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("1:02")
                .unwrap()
                .name("hour")
                .unwrap()
                .as_str(),
            "1"
        );
        assert_eq!(
            BUSINESS_HOURS_PATTERN
                .captures("01:2")
                .unwrap()
                .name("minute")
                .unwrap()
                .as_str(),
            "2"
        );
        assert!(!BUSINESS_HOURS_PATTERN.is_match(""));
        assert!(!BUSINESS_HOURS_PATTERN.is_match("1:"));
        assert!(!BUSINESS_HOURS_PATTERN.is_match(":1"));
        assert!(!BUSINESS_HOURS_PATTERN.is_match("a1:2"));
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(HourMinute(0, 0), parse_time(&"0:0".to_string()).unwrap());
        assert_eq!(HourMinute(1, 2), parse_time(&"01:02".to_string()).unwrap());
        assert_eq!(
            HourMinute(23, 59),
            parse_time(&"23:59".to_string()).unwrap()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got hour=99, but hour must be 0-23".to_string()
            },
            parse_time(&"99:0".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got minute=99, but minute must be 0-59".to_string()
            },
            parse_time(&"0:99".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got hour=99, but hour must be 0-23".to_string()
            },
            parse_time(&"99:99".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got hour=24, but hour must be 0-23".to_string()
            },
            parse_time(&"24:0".to_string()).unwrap_err()
        );
        assert_eq!(
            TTError::TTError {
                message: "Got minute=60, but minute must be 0-59".to_string()
            },
            parse_time(&"23:60".to_string()).unwrap_err()
        );
    }
}
