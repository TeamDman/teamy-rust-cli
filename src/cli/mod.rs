pub mod cache;
mod global_args;

use crate::cli::cache::CacheArgs;
use arbitrary::Arbitrary;
use clap::Parser;
use clap::Subcommand;
use eyre::Context;
pub use global_args::*;
use std::ffi::OsString;

#[derive(Parser, Arbitrary, PartialEq, Debug)]
#[clap(version)]
pub struct Cli {
    #[clap(flatten)]
    pub global_args: GlobalArgs,
    #[clap(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn invoke(self) -> eyre::Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("Failed to build tokio runtime")?;
        runtime.block_on(async move { self.command.invoke().await })?;
        Ok(())
    }
}

impl ToArgs for Cli {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        args.extend(self.global_args.to_args());
        args.extend(self.command.to_args());
        args
    }
}

/// A demonstration command line utility
#[derive(Subcommand, Arbitrary, PartialEq, Debug)]
pub enum Command {
    /// Hello-world demonstration commands
    Cache(CacheArgs),
}

impl Command {
    pub async fn invoke(self) -> eyre::Result<()> {
        match self {
            Command::Cache(args) => args.invoke().await,
        }
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            Command::Cache(cache_args) => {
                args.push("cache".into());
                args.extend(cache_args.to_args());
            }
        }
        args
    }
}

/// Trait for converting CLI structures to command line arguments
pub trait ToArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}

// Blanket implementation for references
impl<T: ToArgs> ToArgs for &T {
    fn to_args(&self) -> Vec<OsString> {
        (*self).to_args()
    }
}
