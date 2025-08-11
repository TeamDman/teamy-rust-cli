pub mod elevation_command;
pub mod elevation_check_command;
pub mod elevation_test_command;

use crate::cli::command::elevation::elevation_command::ElevationCommand;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

/// Elevation command arguments container
#[derive(Args, Arbitrary, PartialEq, Debug)]
pub struct ElevationArgs {
    #[clap(subcommand)]
    pub action: ElevationCommand,
}

impl ElevationArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        self.action.invoke()
    }
}

impl ToArgs for ElevationArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.action.to_args()
    }
}
