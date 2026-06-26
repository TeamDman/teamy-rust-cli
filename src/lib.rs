#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub mod cancellation;
pub mod cli;
pub mod logging_init;
pub mod paths;

use crate::cli::Cli;
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;

/// Version string combining package version, git repository metadata, and build time.
fn version() -> String {
    let built_at = option_env!("BUILD_TIMESTAMP_UNIX")
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
        .map_or_else(
            || "unknown build time".to_string(),
            |timestamp| {
                timestamp
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S %Z")
                    .to_string()
            },
        );

    format!(
        "{} (repo {}, branch {}, rev {}, worktree {}, built {})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_REPOSITORY_URL"),
        env!("GIT_BRANCH"),
        env!("GIT_REVISION"),
        env!("GIT_WORKTREE_STATUS"),
        built_at,
    )
}

/// Entrypoint for the program.
///
/// # Errors
///
/// This function will return an error if `color_eyre` installation, CLI parsing, logging initialization, command execution, or command output rendering fails.
///
/// # Panics
///
/// Panics if the CLI schema is invalid (should never happen with correct code).
pub fn main() -> eyre::Result<()> {
    // Install color_eyre for better error reports
    color_eyre::install()?;
    let cancellation_token = crate::cancellation::CtrlCHandler::default().install()?;

    #[cfg(windows)]
    {
        // Enable ANSI support on Windows
        // This fails in a pipe scenario, so we ignore the error
        let _ = teamy_windows::console::enable_ansi_support();

        // Warn if UTF-8 is not enabled on Windows
        #[cfg(windows)]
        teamy_windows::string::warn_if_utf8_not_enabled();
    };

    // Parse command line arguments using figue
    // unwrap() is figue's intended CLI entry behavior:
    // it exits with proper codes for --help/--version/completions/parse-errors.
    let version = version();

    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |cli| cli.args_os(std::env::args_os().skip(1)).strict())
            .help(move |help| {
                // TODO(template): replace the implementation git URL with the generated repository path.
                help.version(version)
                    .include_implementation_source_file(true)
                    .include_implementation_git_url("TeamDman/teamy-rust-cli", env!("GIT_REVISION"))
            })
            .build(),
    )
    .run()
    .unwrap();

    // Initialize logging
    logging_init::init_logging(&cli.global_args, cancellation_token.clone())?;

    // Invoke whatever command was requested and render its output once at the top level
    let requested_output_format = cli.global_args.output_format;
    let output = cli.invoke(cancellation_token.clone())?;
    cancellation_token.bail_if_cancelled()?;
    output.emit(requested_output_format)?;
    cancellation_token.bail_if_cancelled()?;
    Ok(())
}
