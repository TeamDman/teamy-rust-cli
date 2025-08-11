use std::ffi::OsString;

use crate::cli::command::elevation::elevation_check_command::ElevationCheckArgs;
use crate::cli::command::elevation::elevation_test_command::ElevationTestArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;

/// Administrative privilege operations
#[derive(Subcommand, Clone, Arbitrary, PartialEq, Debug)]
pub enum ElevationCommand {
    /// Check if the current process is running with administrator privileges
    Check(ElevationCheckArgs),
    /// Test elevation functionality by relaunching with administrator privileges
    Test(ElevationTestArgs),
}

impl ElevationCommand {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            ElevationCommand::Check(args) => args.run(),
            ElevationCommand::Test(args) => args.run(),
        }
    }
}

impl ToArgs for ElevationCommand {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            ElevationCommand::Check(check_args) => {
                args.push("check".into());
                args.extend(check_args.to_args());
            }
            ElevationCommand::Test(test_args) => {
                args.push("test".into());
                args.extend(test_args.to_args());
            }
        }
        args
    }
}
