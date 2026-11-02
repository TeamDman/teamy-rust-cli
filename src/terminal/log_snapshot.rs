use super::TerminalLogEntry;

/// A bounded snapshot and cumulative loss counts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalLogSnapshot {
    pub entries: Vec<TerminalLogEntry>,
    pub evicted: u64,
    pub truncated: u64,
    pub lock_failures: u64,
}
