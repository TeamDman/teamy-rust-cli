pub mod cli;
pub mod tracing;

use clap::CommandFactory;
use clap::FromArgMatches;

use crate::cli::Cli;

// Entrypoint for the program to reduce coupling to the name of this crate.
pub fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::command();
    let cli = Cli::from_arg_matches(&cli.get_matches())?;

    tracing::init_tracing(cli.global_args.log_level(), cli.global_args.json_log_behaviour())?;

    cli.invoke()?;
    Ok(())
}
