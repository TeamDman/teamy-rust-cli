pub mod command;
pub mod global_args;
pub mod to_args;
use crate::cli::command::Command;
use crate::cli::global_args::GlobalArgs;
use arbitrary::Arbitrary;
use clap::Parser;
use std::ffi::OsString;
use to_args::Invocable;
use to_args::ToArgs;

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
        self.command.invoke()
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

impl Invocable for Cli {
    fn executable(&self) -> std::path::PathBuf {
        std::env::current_exe().expect("Failed to get current executable path")
    }

    fn args(&self) -> Vec<OsString> {
        self.to_args()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::Arbitrary;
    use clap::Parser;

    #[test]
    fn fuzz_cli_args_roundtrip() {
        // Generate 100 arbitrary CLI instances and test roundtrip conversion
        let mut data = vec![42u8; 1024]; // Create owned data
        let mut rng = arbitrary::Unstructured::new(&data);

        for i in 0..100 {
            // Generate an arbitrary CLI instance
            let cli = match Cli::arbitrary(&mut rng) {
                Ok(cli) => cli,
                Err(_) => {
                    // If we run out of data, refresh with new seed
                    data = vec![i as u8; 1024];
                    rng = arbitrary::Unstructured::new(&data);
                    Cli::arbitrary(&mut rng).expect("Failed to generate CLI instance")
                }
            };

            // Convert CLI to args
            let args = cli.to_args();

            // Create command line with executable name
            let mut full_args = vec!["test-exe".into()];
            full_args.extend(args);

            // Parse back from args
            let parsed_cli = match Cli::try_parse_from(&full_args) {
                Ok(parsed) => parsed,
                Err(e) => {
                    panic!(
                        "Failed to parse CLI args on iteration {}: {}\nOriginal CLI: {:?}\nArgs: {:?}",
                        i, e, cli, full_args
                    );
                }
            };

            // Check equality
            if cli != parsed_cli {
                panic!(
                    "CLI roundtrip failed on iteration {}:\nOriginal: {:?}\nParsed: {:?}\nArgs: {:?}",
                    i, cli, parsed_cli, full_args
                );
            }
        }
    }

    #[test]
    fn fuzz_cli_args_consistency() {
        // Test that the same CLI instance always produces the same args
        let mut data = vec![123u8; 1024]; // Create owned data
        let mut rng = arbitrary::Unstructured::new(&data);

        for i in 0..50 {
            let cli = match Cli::arbitrary(&mut rng) {
                Ok(cli) => cli,
                Err(_) => {
                    data = vec![(i * 2) as u8; 1024];
                    rng = arbitrary::Unstructured::new(&data);
                    Cli::arbitrary(&mut rng).expect("Failed to generate CLI instance")
                }
            };

            let args1 = cli.to_args();
            let args2 = cli.to_args();

            assert_eq!(
                args1, args2,
                "CLI.to_args() should be deterministic for iteration {}",
                i
            );
        }
    }
}
