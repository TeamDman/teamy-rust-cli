use crate::cli::home::path::show::HomePathShowArgs;
use eyre::Result;
use facet::Facet;
use figue as args;

/// Home path commands.
#[derive(Facet, Debug)]
pub struct HomePathArgs {
    /// The home path subcommand to run.
    #[facet(args::subcommand)]
    pub command: HomePathCommand,
}

/// Home path subcommands.
#[derive(Facet, Debug)]
#[repr(u8)]
pub enum HomePathCommand {
    /// Show the home path.
    Show(HomePathShowArgs),
}

impl HomePathArgs {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            HomePathCommand::Show(args) => args.invoke().await?,
        }

        Ok(())
    }
}
