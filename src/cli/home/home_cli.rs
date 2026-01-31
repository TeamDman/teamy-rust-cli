use crate::cli::home::path::HomePathArgs;
use eyre::Result;
use facet::Facet;
use figue as args;

/// Home-related commands.
#[derive(Facet, Debug)]
pub struct HomeArgs {
    /// The home subcommand to run.
    #[facet(args::subcommand)]
    pub command: HomeCommand,
}

/// Home subcommands.
#[derive(Facet, Debug)]
#[repr(u8)]
pub enum HomeCommand {
    /// Show or manage home paths.
    Path(HomePathArgs),
}

impl HomeArgs {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            HomeCommand::Path(args) => args.invoke().await?,
        }

        Ok(())
    }
}
