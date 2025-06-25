use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Layer, Registry};

pub fn init_logging(
    state_directory: &str,
    filter: LevelFilter,
) -> (impl SubscriberExt + use<>, (WorkerGuard, WorkerGuard)) {
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("terrainiumd")
        .filename_suffix("log")
        .build(state_directory)
        .expect("log file appender to be configured");
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(appender);
    let (non_blocking_stdout, out_guard) = tracing_appender::non_blocking(std::io::stdout());

    let timer = fmt::time::LocalTime::rfc_3339();
    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .with_writer(non_blocking_file)
                .with_timer(timer.clone())
                .with_ansi(false)
                .with_target(false)
                .with_filter(filter),
        )
        .with(
            fmt::Layer::default()
                .with_writer(non_blocking_stdout)
                .with_timer(timer)
                .with_filter(filter),
        );

    // return guards to keep subscriber from dropping
    (subscriber, (file_guard, out_guard))
}
