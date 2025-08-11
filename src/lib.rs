pub mod cli;
pub mod windows;
use clap::CommandFactory;
use clap::FromArgMatches;
use tracing::Level;
use tracing::debug;

use crate::cli::Cli;
use crate::windows::console_reuse::reuse_console_if_requested;

/// Initialize tracing subscriber with the given log level.
/// In debug builds, include file and line number without timestamp.
/// In release builds, include timestamp and log level.
pub fn init_tracing(level: Level) {
    let builder = tracing_subscriber::fmt().with_max_level(level);
    #[cfg(debug_assertions)]
    let subscriber = builder
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .without_time()
        .finish();
    #[cfg(not(debug_assertions))]
    let subscriber = builder.finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    debug!("Tracing initialized with level: {:?}", level);
}

// Entrypoint for the program to reduce coupling to the name of this crate.
pub fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::command();
    let cli = Cli::from_arg_matches(&cli.get_matches())?;

    reuse_console_if_requested(&cli.global_args);
    init_tracing(cli.global_args.log_level());

    cli.invoke()?;
    Ok(())
}
