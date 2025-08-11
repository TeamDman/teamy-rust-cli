use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

use crate::cli::to_args::ToArgs;

#[derive(Args, Default, Arbitrary, PartialEq, Debug)]
pub struct GlobalArgs {
    /// Enable debug logging
    #[clap(long, global = true)]
    pub debug: bool,

    /// Console PID for console reuse (hidden)
    #[clap(long, hide = true, global = true)]
    pub console_pid: Option<u32>,
}

impl GlobalArgs {
    pub fn log_level(&self) -> tracing::Level {
        if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }
}

impl ToArgs for GlobalArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if self.debug {
            args.push("--debug".into());
        }
        if let Some(pid) = self.console_pid {
            args.push("--console-pid".into());
            args.push(pid.to_string().into());
        }
        args
    }
}
