use crate::cli::global_args::GlobalArgs;
use crate::cli::log_format::LogFormat;
use eyre::Context;
use std::fs::OpenOptions;
use std::io::IsTerminal;
use std::sync::Mutex;
use teamy_cancellation::CancellationToken;
use tracing::Metadata;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
use tracing_subscriber::filter::FilterExt;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::prelude::*;

fn exclude_tracy_frame_mark(meta: &Metadata<'_>) -> bool {
    meta.fields().field("tracy.frame_mark").is_none()
}

/// Resolve stderr independently of the optional file sink.
#[must_use]
fn stderr_filter_directive<'a>(args: &'a GlobalArgs, inherited: Option<&'a str>) -> &'a str {
    args.log_filter.as_deref().unwrap_or_else(|| {
        if args.debug {
            "debug"
        } else {
            inherited.unwrap_or("info")
        }
    })
}

fn stderr_event(meta: &Metadata<'_>) -> bool {
    exclude_tracy_frame_mark(meta) && meta.target() != "teamy::failure_file"
}

fn parse_filter(value: &str) -> eyre::Result<EnvFilter> {
    EnvFilter::builder()
        .with_regex(false)
        .parse(value)
        .map_err(Into::into)
}

#[cfg(all(feature = "tracy", not(test)))]
fn tracy_log_filter_directives() -> [&'static str; 7] {
    [
        "trace",
        "cranelift_codegen=warn",
        "cranelift_frontend=warn",
        "cranelift_jit=warn",
        "cranelift_native=warn",
        "regalloc2=warn",
        "wasmtime=warn",
    ]
}

/// Initialize independently filtered and formatted diagnostic sinks.
///
/// # Errors
/// Returns invalid filters, file-open failures, or subscriber installation failures.
/// All filters are validated before file effects. Existing files are appended.
pub fn init_logging(args: &GlobalArgs, cancellation: CancellationToken) -> eyre::Result<()> {
    build_subscriber(args, cancellation, BoxMakeWriter::new(std::io::stderr))?
        .try_init()
        .wrap_err("cannot install tracing subscriber")?;
    tracing::debug!("Tracing initialized");
    Ok(())
}

fn build_subscriber(
    args: &GlobalArgs,
    cancellation: CancellationToken,
    stderr: BoxMakeWriter,
) -> eyre::Result<impl tracing::Subscriber + Send + Sync + 'static> {
    let inherited = std::env::var("RUST_LOG").ok();
    let stderr_filter = parse_filter(stderr_filter_directive(args, inherited.as_deref()))
        .wrap_err("invalid stderr log filter")?;
    let file_filter = parse_filter(args.log_file_filter.as_deref().unwrap_or("debug"))
        .wrap_err("invalid file log filter")?;
    let stderr_diagnostics = teamy_tracing::Diagnostics::default();
    let stderr_layer = match args.log_format.unwrap_or_default() {
        LogFormat::Text => tracing_subscriber::fmt::layer()
            .log_internal_errors(false)
            .pretty()
            .without_time()
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(stderr)
            .with_filter(stderr_filter.and(filter_fn(stderr_event)))
            .boxed(),
        LogFormat::Ndjson => tracing_subscriber::fmt::layer()
            .log_internal_errors(false)
            .fmt_fields(teamy_tracing::FacetJsonFields::new(
                stderr_diagnostics.clone(),
            ))
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .event_format(
                teamy_tracing::FacetJson::default()
                    .with_diagnostics(stderr_diagnostics)
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(true),
            )
            .with_writer(stderr)
            .with_filter(stderr_filter.and(filter_fn(stderr_event)))
            .boxed(),
    };
    let subscriber = Registry::default()
        .with(stderr_layer)
        .with(args.stop_after.stop_after_span_layer(cancellation));
    let file_layer = if let Some(path) = args.log_file.as_deref() {
        let file_diagnostics = teamy_tracing::Diagnostics::default();
        let path = std::path::Path::new(path);
        eyre::ensure!(!path.is_dir(), "--log-file requires an explicit file path");
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).wrap_err("cannot create log parent directory")?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .wrap_err("cannot open log destination")?;
        // The guard lives for the formatter's complete record, rather than just handle cloning.
        let writer = BoxMakeWriter::new(Mutex::new(file));
        let layer = match args.log_file_format.unwrap_or(LogFormat::Ndjson) {
            LogFormat::Text => tracing_subscriber::fmt::layer()
                .log_internal_errors(false)
                .with_ansi(false)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(writer)
                .with_span_events(FmtSpan::CLOSE)
                .with_filter(file_filter.and(filter_fn(exclude_tracy_frame_mark)))
                .boxed(),
            LogFormat::Ndjson => tracing_subscriber::fmt::layer()
                .log_internal_errors(false)
                .fmt_fields(teamy_tracing::FacetJsonFields::new(
                    file_diagnostics.clone(),
                ))
                .with_ansi(false)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(writer)
                .with_span_events(FmtSpan::CLOSE)
                .event_format(
                    teamy_tracing::FacetJson::default()
                        .with_diagnostics(file_diagnostics)
                        .with_file(true)
                        .with_line_number(true)
                        .with_target(true),
                )
                .with_filter(file_filter.and(filter_fn(exclude_tracy_frame_mark)))
                .boxed(),
        };
        Some(layer)
    } else {
        None
    };
    let subscriber = subscriber.with(file_layer);
    #[cfg(all(feature = "tracy", not(test)))]
    let subscriber = subscriber.with(
        tracing_tracy::TracyLayer::default()
            .with_filter(parse_filter(&tracy_log_filter_directives().join(","))?),
    );
    Ok(subscriber)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, facet::Facet)]
    struct Line {
        level: String,
    }
    #[test]
    fn stderr_filter_precedence_is_explicit_then_debug_then_environment() {
        let mut args = GlobalArgs::default();
        assert_eq!(stderr_filter_directive(&args, None), "info");
        assert_eq!(stderr_filter_directive(&args, Some("warn")), "warn");
        args.debug = true;
        assert_eq!(stderr_filter_directive(&args, Some("off")), "debug");
        args.log_filter = Some("error,fixture=trace".into());
        assert_eq!(
            stderr_filter_directive(&args, Some("off")),
            "error,fixture=trace"
        );
    }

    #[test]
    fn concurrent_file_records_remain_complete_and_independent() {
        let path = std::env::temp_dir().join(format!(
            "teamy-records-{}-{}.ndjson",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let args = GlobalArgs {
            log_filter: Some("off".into()),
            log_file: Some(path.to_string_lossy().into_owned()),
            ..GlobalArgs::default()
        };
        let subscriber = build_subscriber(
            &args,
            CancellationToken::new(),
            BoxMakeWriter::new(std::io::sink),
        )
        .unwrap();
        let dispatch = tracing::Dispatch::new(subscriber);
        std::thread::scope(|scope| {
            for worker in 0..4 {
                let dispatch = dispatch.clone();
                scope.spawn(move || {
                    tracing::dispatcher::with_default(&dispatch, || {
                        for sequence in 0..40 {
                            tracing::debug!(worker, sequence, "complete record");
                        }
                    });
                });
            }
        });

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 160);
        for line in text.lines() {
            assert_eq!(facet_json::from_str::<Line>(line).unwrap().level, "DEBUG");
        }
        drop(dispatch);
        std::fs::remove_file(path).unwrap();
    }
}
