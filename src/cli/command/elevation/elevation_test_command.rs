use crate::cli::Cli;
use crate::cli::command::Command;
use crate::cli::command::elevation::ElevationArgs;
use crate::cli::command::elevation::elevation_check_command::ElevationCheckArgs;
use crate::cli::command::elevation::elevation_command::ElevationCommand;
use crate::cli::global_args::GlobalArgs;
use crate::cli::to_args::ToArgs;
use crate::windows::win_elevation::is_elevated;
use crate::windows::win_elevation::run_as_admin;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::eyre;
use std::ffi::OsString;
use tracing::info;
use tracing::warn;

/// Arguments for testing elevation functionality
#[derive(Args, Clone, Arbitrary, PartialEq, Debug)]
pub struct ElevationTestArgs {}

impl ElevationTestArgs {
    pub fn run(self) -> eyre::Result<()> {
        if is_elevated() {
            info!("Already running as elevated, elevation test successful!");
            return Ok(());
        }

        warn!("Not elevated. Testing relaunch as administrator...");

        // build the CLI that the elevated instance must execute
        let check_cli = Cli {
            global_args: GlobalArgs {
                console_pid: Some(std::process::id()),
                ..Default::default()
            },
            command: Command::Elevation(ElevationArgs {
                action: ElevationCommand::Check(ElevationCheckArgs {}),
            }),
        };

        info!("Relaunching as administrator to run elevation check...");
        match run_as_admin(&check_cli) {
            Ok(child) => {
                info!("Spawned elevated process - waiting for it to finish…");
                let exit_code = child.wait()?;
                info!("Elevated process exited with code {exit_code}");
                std::process::exit(exit_code as i32);
            }
            Err(e) => Err(eyre!("Failed to relaunch as administrator: {e}")),
        }
    }
}

impl ToArgs for ElevationTestArgs {
    fn to_args(&self) -> Vec<OsString> {
        // No additional args for test command
        Vec::new()
    }
}
