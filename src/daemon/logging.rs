use crate::common::constants::TERRAINIUMD_TMP_DIR;
use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Layer, Registry};

pub fn init_logging(filter: LevelFilter) -> (impl SubscriberExt, (WorkerGuard, WorkerGuard)) {
    let appender = tracing_appender::rolling::daily(TERRAINIUMD_TMP_DIR, "terrainiumd.log");
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(appender);
    let (non_blocking_stdout, out_guard) = tracing_appender::non_blocking(std::io::stdout());

    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .with_writer(non_blocking_file)
                .with_ansi(false)
                .with_target(false)
                .with_filter(filter),
        )
        .with(
            fmt::Layer::default()
                .with_writer(non_blocking_stdout)
                .with_filter(filter),
        );

    // return guards to keep subscriber from dropping
    (subscriber, (file_guard, out_guard))
}
