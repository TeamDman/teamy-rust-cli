use super::TerminalLogBuffer;
use std::io::Write;
use std::io::{self};

/// Captures one formatted tracing record; its buffer has an explicit byte limit.
#[derive(Debug)]
pub struct TerminalLogWriter {
    buffer: TerminalLogBuffer,
    bytes: Vec<u8>,
    truncated: bool,
}

impl TerminalLogWriter {
    pub(crate) fn new(buffer: TerminalLogBuffer) -> Self {
        Self {
            buffer,
            bytes: Vec::new(),
            truncated: false,
        }
    }
}

impl Write for TerminalLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let available = self
            .buffer
            .max_record_bytes()
            .saturating_sub(self.bytes.len());
        let count = buf.len().min(available);
        self.bytes.extend_from_slice(&buf[..count]);
        self.truncated |= count < buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for TerminalLogWriter {
    fn drop(&mut self) {
        if !self.bytes.is_empty() {
            self.buffer.push(&self.bytes, self.truncated);
        }
    }
}
