//! Reviewed failure contexts. Neither Display nor Debug contains caller-controlled data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafeFailure {
    Configuration,
    InvalidInput,
    Runtime,
    GeneratedFileConflict,
}
impl SafeFailure {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Configuration => {
                "[configuration] Cannot initialize logging; check filters and the log-file path"
            }
            Self::InvalidInput => {
                "[invalid-input] Invalid command options or request; check command --help and input format"
            }
            Self::Runtime => {
                "[runtime] Cannot initialize command runtime; check available system resources and diagnostic events"
            }
            Self::GeneratedFileConflict => {
                "Destination contains generated files. Re-run with --force for intentional overwrite"
            }
        }
    }
}
impl std::fmt::Display for SafeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}
impl std::error::Error for SafeFailure {}
