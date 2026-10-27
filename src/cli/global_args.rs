//! Global arguments that apply to all commands.

use crate::cli::log_format::LogFormat;
use crate::cli::output::OutputFormat;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};
use teamy_cancellation::StopAfterArgs;

/// Global arguments that apply to all commands.
#[derive(Facet, Arbitrary, Debug, Default, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct GlobalArgs {
    /// Select stderr DEBUG unless an explicit filter is provided; overrides `RUST_LOG`.
    #[facet(args::named, default)]
    pub debug: bool,

    /// Tracing filter directive, also accepted as `--log-level`.
    #[facet(args::named, args::alias = "log-level")]
    #[arbitrary(default)]
    pub log_filter: Option<String>,

    /// Stderr encoding: text (default) or ndjson.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub log_format: Option<LogFormat>,

    /// File filter directives, independent of stderr and `RUST_LOG`; default debug.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub log_file_filter: Option<String>,

    /// File encoding: ndjson (default) or text.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub log_file_format: Option<LogFormat>,

    /// Append diagnostics to this explicit file path. Omitted means no file sink.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub log_file: Option<String>,
    /// Request graceful cancellation from tracing/Tracy spans, log messages, or elapsed time.
    #[facet(flatten, default)]
    #[arbitrary(default)]
    pub stop_after: StopAfterArgs,

    /// Render command output as `text`, `json`, or `csv`.
    ///
    /// If omitted, the generated CLI uses `text` for interactive terminals and
    /// `json` when stdout is redirected.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub output_format: Option<OutputFormat>,
}
