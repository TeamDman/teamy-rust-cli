pub mod hello;

use crate::cli::command::hello::HelloArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use std::ffi::OsString;

/// A demonstration command line utility
#[derive(Subcommand, Arbitrary, PartialEq, Debug)]
pub enum Command {
    /// Hello-world demonstration commands
    Hello(HelloArgs),
}

impl Command {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            Command::Hello(args) => args.invoke(),
        }
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            Command::Hello(hello_args) => {
                args.push("hello".into());
                args.extend(hello_args.to_args());
            }
        }
        args
    }
}
