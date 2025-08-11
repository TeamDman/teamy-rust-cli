pub mod elevation;
pub mod hello;

use crate::cli::command::elevation::ElevationArgs;
use crate::cli::command::hello::HelloArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use std::ffi::OsString;

/// A demonstration command line utility
#[derive(Subcommand, Arbitrary, PartialEq, Debug)]
pub enum Command {
    /// Administrative privilege elevation utilities
    Elevation(ElevationArgs),
    /// Hello-world demonstration commands
    Hello(HelloArgs),
}

impl Command {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            Command::Elevation(args) => args.invoke(),
            Command::Hello(args) => args.invoke(),
        }
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            Command::Elevation(elevation_args) => {
                args.push("elevation".into());
                args.extend(elevation_args.to_args());
            }
            Command::Hello(hello_args) => {
                args.push("hello".into());
                args.extend(hello_args.to_args());
            }
        }
        args
    }
}
