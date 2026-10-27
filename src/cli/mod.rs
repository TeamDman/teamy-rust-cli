pub mod cache;
pub mod global_args;
pub mod home;
pub mod init;
pub mod log_format;
pub mod output;
mod version;

use crate::cli::cache::CacheArgs;
use crate::cli::global_args::GlobalArgs;
use crate::cli::home::HomeArgs;
use crate::cli::init::InitArgs;
use crate::cli::output::CliOutput;
use crate::elapsed_duration;
use crate::logging_init;
use crate::safe_failure::SafeFailure;
#[cfg(windows)]
use crate::windows_startup;
use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};
use std::io;
use std::io::IsTerminal;
use teamy_cancellation::CancellationToken;
use teamy_cancellation::CtrlCHandler;

/// A demonstration command line utility.
///
/// TODO(template): replace this top-level CLI description with the generated project's real behavior.
///
/// Environment variables:
/// TODO(template): replace the environment variable names below with project-specific names.
/// - `APP_HOME_DIR` overrides the resolved application home directory.
/// - `APP_CACHE_DIR` overrides the resolved cache directory.
/// - `RUST_LOG` provides a tracing filter when `--log-filter` is omitted.
#[derive(Facet, Arbitrary, Debug)]
pub struct Cli {
    /// Global arguments (`debug`, `log_filter`, `log_file`).
    #[facet(flatten)]
    pub global_args: GlobalArgs,

    /// Standard CLI options (help, version, completions).
    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,

    /// The command to run.
    #[facet(args::subcommand)]
    pub command: Command,
}

impl PartialEq for Cli {
    fn eq(&self, other: &Self) -> bool {
        // Ignore builtins in comparison since FigueBuiltins doesn't implement PartialEq
        self.global_args == other.global_args && self.command == other.command
    }
}

impl Cli {
    /// # Errors
    ///
    /// This function will return an error if the tokio runtime cannot be built or if the command fails.
    pub fn invoke(self, cancellation_token: CancellationToken) -> eyre::Result<CliOutput> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err(SafeFailure::Runtime)?;
        runtime.block_on(async move { self.command.invoke(cancellation_token).await })
    }
}

/// A demonstration command line utility.
///
/// TODO(template): replace these example command groups with the generated project's actual command surface.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Cache-related commands.
    Cache(CacheArgs),
    /// Home-related commands.
    Home(HomeArgs),
    /// Scaffold a new Teamy Rust CLI repository.
    Init(InitArgs),
}

impl Command {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self, cancellation_token: CancellationToken) -> eyre::Result<CliOutput> {
        // Builtin commands perform synchronous mutations within a poll. Future async
        // commands must be safe to abandon at await points before joining this wrapper.
        cancellation_token
            .cancellable_scope(|scope| async move {
                scope.bail_if_cancelled()?;
                match self {
                    Command::Cache(args) => args.invoke().await,
                    Command::Home(args) => args.invoke().await,
                    Command::Init(args) => args.invoke().await,
                }
            })
            .await
    }
}

impl Cli {
    pub(crate) fn parse() -> eyre::Result<Self> {
        let version = Self::version();

        let arguments = std::env::args_os()
            .skip(1)
            .map(|argument| {
                argument
                    .into_string()
                    .map_err(|_non_unicode_argument| SafeFailure::InvalidInput)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let cli: figue::DriverOutcome<Cli> = figue::Driver::new(
            figue::builder::<Cli>()
                .expect("schema should be valid")
                .cli(move |cli| cli.args(arguments).strict())
                .help(move |help| {
                    let help = help
                        .version(version)
                        .include_implementation_source_file(true);
                    match env!("CARGO_PKG_REPOSITORY").strip_prefix("https://github.com/") {
                        Some(repository) if !repository.is_empty() => {
                            help.include_implementation_github_url(repository, env!("GIT_REVISION"))
                        }
                        _ => help,
                    }
                })
                .build(),
        )
        .run();
        let cli = Self::finish_parse(cli)?;

        Ok(cli)
    }

    pub(crate) fn run(self) -> eyre::Result<()> {
        let cli = self;
        let cancellation_token = CtrlCHandler::default()
            .install()
            .wrap_err(SafeFailure::Runtime)?;
        let _stop_after_duration_thread = cli
            .global_args
            .stop_after
            .start_stop_after_duration_thread(cancellation_token.clone())
            .wrap_err(SafeFailure::InvalidInput)?;

        // Initialize logging
        logging_init::init_logging(&cli.global_args, cancellation_token.clone())
            .wrap_err(SafeFailure::Configuration)?;

        #[cfg(windows)]
        {
            // Enable ANSI support on Windows
            // This fails in a pipe scenario, so we ignore the error
            let _ = windows_startup::enable_ansi_support();

            // Warn if UTF-8 is not enabled on Windows
            #[cfg(windows)]
            windows_startup::warn_if_utf8_not_enabled();
        };

        let command_name = crate::command_path::CommandPath::from_cli(&cli)?.into_string();
        tracing::debug!(command = command_name.as_str(), "Dispatching command");
        let result = (|| -> eyre::Result<()> {
            // Measure command work after logging setup, including output and early returns.
            let _command_span =
                tracing::info_span!("command", command = command_name.as_str()).entered();
            let started = std::time::Instant::now();

            // Invoke whatever command was requested and render its output once at the top level
            let requested_output_format = cli.global_args.output_format;
            let output = cli.invoke(cancellation_token.clone())?;
            let threshold = output.elapsed_duration_warn_threshold;
            cancellation_token.with_scope(|scope| scope.bail_if_cancelled())?;
            let stdout = io::stdout();
            let output_is_terminal = stdout.is_terminal();
            let emitted = output.emit_to(
                &mut stdout.lock(),
                requested_output_format,
                output_is_terminal,
            );
            elapsed_duration::warn_if_exceeded(&command_name, started.elapsed(), threshold);
            emitted?;
            cancellation_token.with_scope(|scope| scope.bail_if_cancelled())?;
            Ok(())
        })();
        tracing::debug!(
            command = command_name.as_str(),
            success = result.is_ok(),
            "Command finished"
        );
        result
    }
    fn finish_parse(
        outcome: figue::DriverOutcome<crate::cli::Cli>,
    ) -> eyre::Result<crate::cli::Cli> {
        match outcome.into_result() {
            Ok(output) => Ok(output.value),
            Err(figue::DriverError::Help { text, .. }) => {
                // Figue's missing-command suggestion can contain supplied values.
                Ok(figue::DriverOutcome::<Cli>::err(figue::DriverError::Help {
                    text,
                    suggestion: None,
                })
                .unwrap())
            }
            Err(error) => {
                // Parse diagnostics can contain the entire argv, including mail bodies or secrets.
                if !error.is_success() {
                    return Err(crate::safe_failure::SafeFailure::InvalidInput.into());
                }
                Ok(figue::DriverOutcome::<crate::cli::Cli>::err(error).unwrap())
            }
        }
    }
}
