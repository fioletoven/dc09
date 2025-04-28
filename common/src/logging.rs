use anyhow::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes new logging to the console and returns worker guard that will flush logs on drop.
pub fn initialize(app_name: &str) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(std::io::stdout());

    let timer = time::format_description::parse("[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second]")?;
    let time_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let timer = tracing_subscriber::fmt::time::OffsetTime::new(time_offset, timer);

    #[cfg(debug_assertions)]
    let env = format!("warn,{}=info", app_name);

    #[cfg(not(debug_assertions))]
    let env = format!("none,{}=info", app_name);

    let env_filter = tracing_subscriber::filter::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::filter::EnvFilter::new(env));

    let stdout = tracing_subscriber::fmt::layer()
        .with_timer(timer)
        .with_ansi(true)
        .with_writer(non_blocking_appender)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stdout).with(ErrorLayer::default()).init();

    Ok(guard)
}
