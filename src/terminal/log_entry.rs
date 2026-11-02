/// One retained human-formatted record, identified within its buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalLogEntry {
    pub sequence: u64,
    pub text: String,
    pub truncated: bool,
}
