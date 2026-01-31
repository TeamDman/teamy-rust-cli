use crate::cli::cache::clean::CacheCleanArgs;
use crate::cli::cache::path::CachePathArgs;
use eyre::Result;
use facet::Facet;
use figue as args;

/// Cache-related commands.
#[derive(Facet, Debug)]
pub struct CacheArgs {
    /// The cache subcommand to run.
    #[facet(args::subcommand)]
    pub command: CacheCommand,
}

/// Cache subcommands.
#[derive(Facet, Debug)]
#[repr(u8)]
pub enum CacheCommand {
    /// Clean the cache.
    Clean(CacheCleanArgs),
    /// Show or manage cache paths.
    Path(CachePathArgs),
}

impl CacheArgs {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            CacheCommand::Clean(args) => args.invoke().await?,
            CacheCommand::Path(args) => args.invoke().await?,
        }

        Ok(())
    }
}
