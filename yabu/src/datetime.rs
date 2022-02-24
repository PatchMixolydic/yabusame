use std::{
    fmt::{self, Debug, Formatter},
    str::FromStr,
};
use time::{format_description, Date, Time};
use tz::TimeZone;

pub fn time_zone_to_str(time_zone: &TimeZone) -> &str {
    time_zone
        .find_current_local_time_type()
        .map(|local| local.time_zone_designation())
        .unwrap_or("???")
}

pub struct DateTime {
    date: Date,
    time: Option<Time>,
    time_zone: TimeZone,
}

impl Debug for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        // `TimeZone`'s `Debug` representation is outrageously long
        f.debug_struct("DateTime")
            .field("date", &self.date)
            .field("time", &self.time)
            .field("time_zone", &time_zone_to_str(&self.time_zone))
            .finish()
    }
}

impl FromStr for DateTime {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: `time`'s parsing is truly arcane; doing this manually be
        // more flexible
        let date_fmt = format_description::parse("[year]-[month]-[day]")?;
        let time_fmt = format_description::parse("[hour padding:none]:[minute][period case_sensitive:false]")?;

        let (date_str, maybe_time_str) = s
            .find(char::is_whitespace)
            .or_else(|| s.find('T'))
            .map(|split_point| {
                let (date_str, time_str) = s.split_at(split_point);
                (date_str, Some(time_str.trim()))
            })
            .unwrap_or((s, None));

        let date = Date::parse(date_str, &date_fmt)?;
        let time = maybe_time_str
            .map(|time_str| Time::parse(time_str, &time_fmt))
            .transpose()?;

        Ok(Self {
            date,
            time,
            time_zone: TimeZone::local()?,
        })
    }
}
