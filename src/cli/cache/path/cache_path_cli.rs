use crate::cli::cache::path::show::CachePathShowArgs;
use eyre::Result;
use facet::Facet;
use figue as args;

/// Cache path commands.
#[derive(Facet, Debug)]
pub struct CachePathArgs {
    /// The cache path subcommand to run.
    #[facet(args::subcommand)]
    pub command: CachePathCommand,
}

/// Cache path subcommands.
#[derive(Facet, Debug)]
#[repr(u8)]
pub enum CachePathCommand {
    /// Show the cache path.
    Show(CachePathShowArgs),
}

impl CachePathArgs {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            CachePathCommand::Show(args) => args.invoke().await?,
        }

        Ok(())
    }
}
