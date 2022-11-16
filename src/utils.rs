
use chrono::{self, NaiveDate}; // for get_utc


pub fn today() -> NaiveDate {
    // Give a NaiveDate for the current local time
    let now = chrono::offset::Local::now();
    let now_str = now.to_string()[0..10].to_string();
    NaiveDate::parse_from_str(&now_str, "%Y-%m-%d").unwrap()
}


