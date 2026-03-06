use crate::cli::global_args::GlobalArgs;
use chrono::Local;
use eyre::bail;
use std::fs::File;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize logging based on the provided configuration.
///
/// # Errors
///
/// This function will return an error if creating the log file or directories fails.
///
/// # Panics
///
/// This function may panic if locking or cloning the log file handle fails.
pub fn init_logging(global_args: &GlobalArgs) -> eyre::Result<()> {
    let subscriber = Registry::default();

    let env_filter_layer = EnvFilter::builder()
        .with_default_directive(match (global_args.debug, global_args.log_filter.as_ref()) {
            (true, None) => LevelFilter::DEBUG.into(),
            (false, None) => LevelFilter::INFO.into(),
            (true, Some(_)) => bail!("cannot specify log filter with --debug"),
            (false, Some(x)) => LevelFilter::from_str(x)?.into(),
        })
        .from_env()?;
    let subscriber = subscriber.with(env_filter_layer);

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_file(cfg!(debug_assertions))
        .with_line_number(cfg!(debug_assertions))
        .with_target(true)
        .with_writer(std::io::stderr)
        .pretty()
        .without_time();
    let subscriber = subscriber.with(stderr_layer);

    let json_log_path = match global_args.log_file.as_ref() {
        None => None,
        Some(path) if std::path::PathBuf::from(path).is_dir() => {
            let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
            let filename = format!("log_{timestamp}.ndjson");
            Some(std::path::PathBuf::from(path).join(filename))
        }
        Some(path) => Some(std::path::PathBuf::from(path)),
    };
    let json_layer = if let Some(ref json_log_path) = json_log_path {
        if let Some(parent) = json_log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(json_log_path)?;
        let file = Arc::new(Mutex::new(file));
        let json_writer = BoxMakeWriter::new(move || {
            file.lock()
                .expect("failed to lock json log file")
                .try_clone()
                .expect("failed to clone json log file handle")
        });

        let json_layer = tracing_subscriber::fmt::layer()
            .event_format(tracing_subscriber::fmt::format().json())
            .with_file(true)
            .with_target(false)
            .with_line_number(true)
            .with_writer(json_writer);
        Some(json_layer)
    } else {
        None
    };
    let subscriber = subscriber.with(json_layer);

    #[cfg(all(feature = "tracy", not(test)))]
    let subscriber = subscriber.with(tracing_tracy::TracyLayer::default());

    if let Err(error) = subscriber.try_init() {
        eprintln!(
            "Failed to initialize tracing subscriber - are you running `cargo test`? If so, multiple test entrypoints may be running from the same process. https://github.com/tokio-rs/console/issues/505 : {error}"
        );
        return Ok(());
    }

    #[cfg(all(feature = "tracy", not(test)))]
    tracing::info!(
        "Tracy profiling layer added, memory usage will increase until a client is connected"
    );

    debug!(
        ?json_log_path,
        debug = global_args.debug,
        "Tracing initialized"
    );
    Ok(())
}
