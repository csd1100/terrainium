pub fn timestamp() -> String {
    if let Ok(now) = time::OffsetDateTime::now_local() {
        now.format(
            &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                .expect("time format to be parsed"),
        )
    } else {
        time::OffsetDateTime::now_utc().format(
            &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                .expect("time format to be parsed"),
        )
    }
    .expect("time to be formatted")
}
