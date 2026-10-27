use crate::cli::Cli;
use crate::cli::log_format::LogFormat;
use crate::failure_record::FailureRecord;
use color_eyre::config::HookBuilder;
use color_eyre::config::PanicHook;
use color_eyre::config::Theme;
use std::io::Write;
use std::sync::Arc;

/// Install error reporting before parsing; use the parsed format for command diagnostics.
#[must_use]
pub fn entrypoint() -> std::process::ExitCode {
    let panic_hook = match install_report_hooks() {
        Ok(hook) => hook,
        Err(error) => return finish_command(Err(error), LogFormat::Text),
    };
    let invocation = match Cli::parse() {
        Ok(invocation) => invocation,
        Err(error) => return finish_command(Err(error), LogFormat::Text),
    };
    let format = invocation.global_args.log_format.unwrap_or_default();
    install_panic_hook(panic_hook, format);
    finish_command(invocation.run(), format)
}

fn install_report_hooks() -> eyre::Result<Arc<PanicHook>> {
    // Reports can also be embedded in NDJSON and log files, so keep their content ANSI-free.
    let (panic_hook, error_hook) = HookBuilder::default()
        .theme(Theme::new())
        .try_into_hooks()?;
    error_hook.install()?;
    let panic_hook = Arc::new(panic_hook);
    install_panic_hook(Arc::clone(&panic_hook), LogFormat::Text);
    Ok(panic_hook)
}

fn install_panic_hook(hook: Arc<PanicHook>, format: LogFormat) {
    std::panic::set_hook(Box::new(move |info| {
        let report = hook.panic_report(info).to_string();
        let _ = report_failure(format, &report);
    }));
}
fn finish_command(result: eyre::Result<()>, format: LogFormat) -> std::process::ExitCode {
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("{error:?}");
            tracing::error!(target: "teamy::failure_file", event_kind = "failure", "{message}");
            let _ = report_failure(format, &message);
            std::process::ExitCode::FAILURE
        }
    }
}

fn report_failure(format: LogFormat, message: &str) -> std::io::Result<()> {
    let mut stderr = std::io::stderr().lock();
    match format {
        LogFormat::Text => writeln!(stderr, "ERROR: {message}"),
        LogFormat::Ndjson => {
            let record = FailureRecord {
                level: "ERROR",
                event_kind: "failure",
                message,
            };
            let encoded = facet_json::to_string(&record).map_err(std::io::Error::other)?;
            writeln!(stderr, "{encoded}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(facet::Facet, Debug)]
    struct Diagnostic {
        message: String,
    }

    #[test]
    fn panic_reports_retain_details_and_ndjson_framing() {
        const MODE: &str = "TEAMY_TEST_PANIC_FORMAT";
        if let Ok(mode) = std::env::var(MODE) {
            let hook = install_report_hooks().unwrap();
            install_panic_hook(
                hook,
                if mode == "ndjson" {
                    LogFormat::Ndjson
                } else {
                    LogFormat::Text
                },
            );
            panic!("fixture panic details");
        }

        // Each hook is process-global; exercise the production setup in isolated children.
        for mode in ["text", "ndjson"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "entrypoint::tests::panic_reports_retain_details_and_ndjson_framing",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env(MODE, mode)
                .output()
                .unwrap();
            assert!(!output.status.success());
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(!stderr.contains('\u{1b}'));
            let message = if mode == "ndjson" {
                assert_eq!(stderr.lines().count(), 1, "{stderr}");
                facet_json::from_str::<Diagnostic>(stderr.trim())
                    .unwrap()
                    .message
            } else {
                stderr
            };
            assert!(message.contains("fixture panic details"), "{message}");
            assert!(message.contains("entrypoint.rs"), "{message}");
        }
    }
}
