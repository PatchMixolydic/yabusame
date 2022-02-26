use time::{format_description, Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use tz::TimeZone;


pub fn offset_date_time_from_str(s: &str) -> Result<OffsetDateTime, String> {
    // TODO: currently, this just treats events with unspecified times as
    // occuring at midnight UTC; the server should probably decide how to
    // handle this instead
 
    let result: anyhow::Result<_> = try {
        // TODO: `time`'s parsing is truly arcane; doing this manually be
        // more flexible
        let date_fmt = format_description::parse("[year]-[month]-[day]")?;
        let time_fmt =
            format_description::parse("[hour padding:none]:[minute][period case_sensitive:false]")?;

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
            .transpose()?
            .unwrap_or(Time::MIDNIGHT);

        // Due to CVE-2020-26235, we can't just use `OffsetDateTime::now_local`.
        // The vulnerability is dodged by using `tz-rs`, which does not call
        // `localtime_r`.
        let offset = UtcOffset::from_whole_seconds(
            TimeZone::local()?
                .find_current_local_time_type()?
                .ut_offset(),
        )?;

        PrimitiveDateTime::new(date, time).assume_offset(offset)
    };

    result.map_err(|err| err.to_string())
}
