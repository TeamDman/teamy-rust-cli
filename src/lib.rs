pub mod cli;
pub mod logging;
pub mod paths;

use crate::cli::Cli;
use clap::CommandFactory;
use clap::FromArgMatches;

// Entrypoint for the program to reduce coupling to the name of this crate.
pub fn main() -> eyre::Result<()> {
    // Install color_eyre for better error reports
    color_eyre::install()?;

    // Parse command line arguments
    let cli = Cli::command();
    let cli = Cli::from_arg_matches(&cli.get_matches())?;

    // Initialize logging
    logging::init_logging(&cli.global_args.logging_config()?)?;

    // Enable ANSI support on Windows
    #[cfg(windows)]
    teamy_windows::console::enable_ansi_support()?;

    // Warn if UTF-8 is not enabled on Windows
    #[cfg(windows)]
    teamy_windows::string::warn_if_utf8_not_enabled();

    // Invoke whatever command was requested
    cli.invoke()?;
    Ok(())
}
