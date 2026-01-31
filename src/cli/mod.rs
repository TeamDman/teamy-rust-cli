pub mod cache;
mod global_args;
pub mod home;

use crate::cli::cache::CacheArgs;
use crate::cli::home::HomeArgs;
use crate::logging::LoggingConfig;
use chrono::Local;
use eyre::Context;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;

/// A demonstration command line utility.
#[derive(Facet, Debug)]
pub struct Cli {
    /// Enable debug logging, including backtraces on panics.
    #[facet(args::named, default)]
    pub debug: bool,

    /// Log level filter directive.
    #[facet(args::named)]
    pub log_filter: Option<String>,

    /// Write structured ndjson logs to this file or directory.
    #[facet(args::named)]
    pub log_file: Option<PathBuf>,

    /// Standard CLI options (help, version, completions).
    #[facet(flatten)]
    pub builtins: FigueBuiltins,

    /// The command to run.
    #[facet(args::subcommand)]
    pub command: Command,
}

impl Cli {
    /// # Errors
    ///
    /// This function will return an error if the tokio runtime cannot be built or if the command fails.
    pub fn invoke(self) -> eyre::Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("Failed to build tokio runtime")?;
        runtime.block_on(async move { self.command.invoke().await })?;
        Ok(())
    }

    /// Get the logging configuration from CLI arguments.
    ///
    /// # Errors
    ///
    /// This function will return an error if the log filter string is invalid.
    pub fn logging_config(&self) -> eyre::Result<LoggingConfig> {
        Ok(LoggingConfig {
            default_directive: match (self.debug, &self.log_filter) {
                (true, _) => LevelFilter::DEBUG,
                (false, Some(filter)) => LevelFilter::from_str(filter)?,
                (false, None) => LevelFilter::INFO,
            }
            .into(),
            json_log_path: match &self.log_file {
                None => None,
                Some(path) if path.is_dir() => {
                    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
                    let filename = format!("log_{timestamp}.ndjson");
                    Some(path.join(filename))
                }
                Some(path) => Some(path.clone()),
            },
        })
    }
}

/// A demonstration command line utility.
#[derive(Facet, Debug)]
#[repr(u8)]
pub enum Command {
    /// Cache-related commands.
    Cache(CacheArgs),
    /// Home-related commands.
    Home(HomeArgs),
}

impl Command {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self {
            Command::Cache(args) => args.invoke().await,
            Command::Home(args) => args.invoke().await,
        }
    }
}
