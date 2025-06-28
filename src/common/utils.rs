use std::fs::create_dir_all;
use std::path::Path;

use anyhow::Context;
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::common::constants::TEST_TIMESTAMP;

pub fn remove_non_numeric(string: &str) -> String {
    let re = Regex::new(r"[^0-9]").unwrap();
    re.replace_all(string, "").to_string()
}

pub fn timestamp() -> String {
    if cfg!(test) {
        TEST_TIMESTAMP.to_string()
    } else if let Ok(now) = time::OffsetDateTime::now_local() {
        now.format(
            &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                .expect("time format to be parsed"),
        )
        .expect("time to be formatted")
    } else {
        time::OffsetDateTime::now_utc()
            .format(
                &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                    .expect("time format to be parsed"),
            )
            .expect("time to be formatted")
    }
}

pub async fn create_file(path: &Path) -> anyhow::Result<File> {
    if path.parent().is_some_and(|parent| !parent.exists()) {
        create_dir_all(path.parent().unwrap()).context(format!(
            "failed to create parent directory {:?}",
            path.parent()
        ))?;
    }

    let file = File::options()
        .create(true)
        .read(true)
        .write(true)
        .append(false)
        .truncate(false)
        .open(path)
        .await
        .context(format!("failed to open file for {path:?})"))?;
    Ok(file)
}

pub async fn write_to_file(file: &mut File, data: String) -> anyhow::Result<()> {
    file.rewind().await.context("failed to rewind the file")?;

    file.set_len(0)
        .await
        .context("failed to truncate the file")?;

    file.write_all(data.as_bytes())
        .await
        .context("failed to write data to the file")?;
    file.flush().await?;

    file.rewind()
        .await
        .context("failed to rewind file the file")?;
    Ok(())
}

pub async fn read_from_file(file: &mut File) -> anyhow::Result<String> {
    file.rewind()
        .await
        .context("failed to rewind file handle")?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .await
        .context("failed to read terrain state")?;

    file.rewind()
        .await
        .context("failed to rewind file handle")?;
    Ok(buf)
}

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/git_hash.txt"));
const BUILD_MODE: &str = if cfg!(debug_assertions) {
    "debug"
} else {
    "release"
};

pub const VERSION_INFO: &str = const_str::concat!("v", VERSION, "-", BUILD_MODE, "+", GIT_HASH);
