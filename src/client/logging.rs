use crate::client::args::{ClientArgs, Verbs};
use tracing::metadata::LevelFilter;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Layer, Registry};

pub fn init_logging(args: &ClientArgs) -> WorkerGuard {
    let level_filter = if matches!(args.command, Some(Verbs::Validate)) {
        // if validate show debug level logs
        LevelFilter::from(Level::DEBUG)
    } else {
        LevelFilter::from(args.options.log_level)
    };

    // need to keep _out_guard in scope till program exits for logger to work
    let (non_blocking_stdout, out_guard) = tracing_appender::non_blocking(std::io::stdout());

    let subscriber = Registry::default().with(
        fmt::Layer::default()
            .with_writer(non_blocking_stdout)
            .with_target(false)
            .with_filter(level_filter),
    );

    if !matches!(args.command, Some(Verbs::Get { debug: false, .. })) {
        // do not print any logs for get command as output will be used by scripts
        tracing::subscriber::set_global_default(subscriber)
            .expect("unable to set global subscriber");
    }

    // return guards to keep subscriber from dropping
    out_guard
}
