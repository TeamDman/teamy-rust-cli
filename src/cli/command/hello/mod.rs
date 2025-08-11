pub mod hello_command;
pub mod hello_greet_command;
pub mod hello_proof_command;
use crate::cli::{command::hello::hello_command::HelloCommand, to_args::ToArgs};
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Elevation command arguments container
#[derive(Args, Arbitrary, PartialEq, Debug)]
pub struct HelloArgs {
    #[clap(subcommand)]
    pub command: HelloCommand,
}

impl HelloArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        self.command.invoke()
    }
}

impl ToArgs for HelloArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}
