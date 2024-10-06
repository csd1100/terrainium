use std::time::SystemTime;
use time::OffsetDateTime;

pub fn timestamp() -> String {
    if let Ok(now) = OffsetDateTime::now_local() {
        now.format(
            &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                .expect("time format to be parsed"),
        )
    } else {
        time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)
    }
    .expect("time to be formatted")
}

pub fn time_to_string(time: SystemTime) -> String {
    let now = OffsetDateTime::from(time);
    now.format(&time::format_description::well_known::Rfc3339)
        .expect("time to be formatted")
}

pub fn string_to_time(time: &str) -> SystemTime {
    OffsetDateTime::parse(time, &time::format_description::well_known::Rfc3339)
        .expect("failed to parse timestamp")
        .into()
}
