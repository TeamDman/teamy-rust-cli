use crate::cli::home::open::HomeOpenArgs;
use crate::cli::home::show::HomeShowArgs;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;

/// Home-related commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct HomeArgs {
    /// The home subcommand to run.
    #[facet(args::subcommand)]
    pub command: HomeCommand,
}

/// Home subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum HomeCommand {
    /// Open the home path in the file manager.
    Open(HomeOpenArgs),
    /// Show the home path.
    Show(HomeShowArgs),
}

impl HomeArgs {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            HomeCommand::Open(args) => args.invoke().await?,
            HomeCommand::Show(args) => args.invoke().await?,
        }

        Ok(())
    }
}
