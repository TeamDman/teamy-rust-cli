use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;

use crate::cli::json_log_behaviour::JsonLogBehaviour;
use crate::cli::to_args::ToArgs;

#[derive(Args, Default, Arbitrary, PartialEq, Debug)]
pub struct GlobalArgs {
    /// Enable debug logging
    #[clap(long, global = true)]
    pub debug: bool,

    /// Emit structured JSON logs alongside stderr output.
    /// Optionally specify a filename; if not provided, a timestamped filename will be generated.
    #[clap(
        long,
        global = true,
        value_name = "FILE",
        num_args = 0..=1,
        default_missing_value = "",
        require_equals = false
    )]
    log_file: Option<String>,
}

impl GlobalArgs {
    pub fn log_level(&self) -> tracing::Level {
        if self.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }

    /// Get the JSON log behaviour based on the --json argument.
    pub fn json_log_behaviour(&self) -> JsonLogBehaviour {
        match &self.log_file {
            None => JsonLogBehaviour::None,
            Some(s) if s.is_empty() => JsonLogBehaviour::SomeAutomaticPath,
            Some(s) => JsonLogBehaviour::Some(s.into()),
        }
    }
}

impl ToArgs for GlobalArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if self.debug {
            args.push("--debug".into());
        }
        match &self.log_file {
            None => {}
            Some(s) if s.is_empty() => {
                args.push("--log-file".into());
            }
            Some(path) => {
                args.push("--log-file".into());
                args.push(path.into());
            }
        }
        args
    }
}
