use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;
use std::io::Write;

use crate::cli::to_args::ToArgs;
use crate::windows::win_elevation::is_elevated;

/// Arguments for checking elevation status
#[derive(Args, Clone, Arbitrary, PartialEq, Debug)]
pub struct ElevationCheckArgs {}

impl ElevationCheckArgs {
    pub fn run(self) -> eyre::Result<()> {
        if is_elevated() {
            println!("Elevated");
            println!("Press Enter to continue...");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
        } else {
            println!("Not Elevated");
        }
        Ok(())
    }
}

impl ToArgs for ElevationCheckArgs {
    fn to_args(&self) -> Vec<OsString> {
        // No additional args for check command
        Vec::new()
    }
}
