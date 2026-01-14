use crate::cli::ToArgs;
use crate::cli::cache::clean::CacheCleanArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;

/// Cache-related commands.
#[derive(Args, Debug, Clone, Arbitrary, PartialEq)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommand,
}

#[derive(Subcommand, Debug, Clone, Arbitrary, PartialEq)]
pub enum CacheCommand {
    Clean(CacheCleanArgs),
}

impl CacheArgs {
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            CacheCommand::Clean(args) => args.invoke().await?,
        }

        Ok(())
    }
}

impl ToArgs for CacheArgs {
    fn to_args(&self) -> Vec<std::ffi::OsString> {
        let mut args = Vec::new();
        match &self.command {
            CacheCommand::Clean(clean_args) => {
                args.push("clean".into());
                args.extend(clean_args.to_args());
            }
        }
        args
    }
}
