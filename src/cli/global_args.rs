//! Global arguments that apply to all commands.

use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};

/// Global arguments that apply to all commands.
#[derive(Facet, Arbitrary, Debug, Default, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct GlobalArgs {
    /// Enable debug logging, including backtraces on panics.
    #[facet(args::named, default)]
    pub debug: bool,

    /// Log level filter directive.
    #[facet(args::named)]
    pub log_filter: Option<String>,

    /// Write structured ndjson logs.
    ///
    /// If a file path is provided, logs are written to that file.
    /// If a directory path is provided, a filename like `log_<timestamp>.ndjson`
    /// is generated in that directory.
    /// If omitted, no JSON log file is written.
    #[facet(args::named)]
    pub log_file: Option<String>,
}