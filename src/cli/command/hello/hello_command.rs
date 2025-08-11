use crate::cli::command::hello::hello_greet_command::HelloGreetArgs;
use crate::cli::command::hello::hello_proof_command::HelloProofArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use std::ffi::OsString;

/// A hello-world command for demonstration purposes
#[derive(Subcommand, Clone, Arbitrary, PartialEq, Debug)]
pub enum HelloCommand {
    /// Greet someone with a friendly message
    Greet(HelloGreetArgs),
    /// Prove that elevation functionality works
    Proof(HelloProofArgs),
}

impl HelloCommand {
    pub fn invoke(self) -> eyre::Result<()> {
        match self {
            HelloCommand::Greet(args) => args.invoke(),
            HelloCommand::Proof(args) => args.invoke(),
        }
    }
}

impl ToArgs for HelloCommand {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            HelloCommand::Greet(check_args) => {
                args.push("greet".into());
                args.extend(check_args.to_args());
            }
            HelloCommand::Proof(test_args) => {
                args.push("proof".into());
                args.extend(test_args.to_args());
            }
        }
        args
    }
}
