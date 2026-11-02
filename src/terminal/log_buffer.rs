use super::TerminalLogEntry;
use super::TerminalLogSnapshot;
use super::TerminalLogWriter;
use std::collections::VecDeque;
use std::io::Write;
use std::io::{self};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug)]
struct State {
    next: u64,
    evicted: u64,
    truncated: u64,
    entries: VecDeque<TerminalLogEntry>,
}

/// Bounded records for a caller-owned tracing layer and inline terminal.
#[derive(Clone, Debug)]
pub struct TerminalLogBuffer {
    state: Arc<Mutex<State>>,
    lock_failures: Arc<AtomicU64>,
    max_records: usize,
    max_record_bytes: usize,
}

impl TerminalLogBuffer {
    /// # Errors
    /// Returns an error when a limit is zero.
    pub fn new(max_records: usize, max_record_bytes: usize) -> io::Result<Self> {
        if max_records == 0 || max_record_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log limits must be nonzero",
            ));
        }
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                next: 0,
                evicted: 0,
                truncated: 0,
                entries: VecDeque::new(),
            })),
            lock_failures: Arc::new(AtomicU64::new(0)),
            max_records,
            max_record_bytes,
        })
    }

    /// Read at most `limit` records; records are not removed until acknowledged.
    /// # Errors
    /// Returns an error if the buffer lock is poisoned.
    pub fn snapshot(&self, limit: usize) -> io::Result<TerminalLogSnapshot> {
        let state = self
            .state
            .lock()
            .map_err(|_poisoned| io::Error::other("terminal log buffer poisoned"))?;
        Ok(TerminalLogSnapshot {
            entries: state.entries.iter().take(limit).cloned().collect(),
            evicted: state.evicted,
            truncated: state.truncated,
            lock_failures: self.lock_failures.load(Ordering::Relaxed),
        })
    }

    /// # Errors
    /// Returns an error if the buffer lock is poisoned.
    pub fn acknowledge(&self, through: u64) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_poisoned| io::Error::other("terminal log buffer poisoned"))?;
        while state
            .entries
            .front()
            .is_some_and(|entry| entry.sequence <= through)
        {
            state.entries.pop_front();
        }
        Ok(())
    }

    /// Replay to a supplied sink, acknowledging each record only after flush succeeds.
    /// A partial write before an error may be repeated on retry.
    /// # Errors
    /// Returns an error on buffer or sink failure.
    pub fn replay(&self, mut writer: impl Write) -> io::Result<()> {
        for entry in self.snapshot(usize::MAX)?.entries {
            writer.write_all(entry.text.as_bytes())?;
            writer.flush()?;
            self.acknowledge(entry.sequence)?;
        }
        Ok(())
    }

    pub(crate) fn max_record_bytes(&self) -> usize {
        self.max_record_bytes
    }
    pub(crate) fn push(&self, bytes: &[u8], truncated: bool) {
        let Ok(mut state) = self.state.lock() else {
            self.lock_failures.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if state.entries.len() == self.max_records {
            state.entries.pop_front();
            state.evicted = state.evicted.saturating_add(1);
        }
        state.next = state.next.saturating_add(1);
        if truncated {
            state.truncated = state.truncated.saturating_add(1);
        }
        let sequence = state.next;
        state.entries.push_back(TerminalLogEntry {
            sequence,
            text: String::from_utf8_lossy(bytes).into_owned(),
            truncated,
        });
    }
}

impl Default for TerminalLogBuffer {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                next: 0,
                evicted: 0,
                truncated: 0,
                entries: VecDeque::new(),
            })),
            lock_failures: Arc::new(AtomicU64::new(0)),
            max_records: 512,
            max_record_bytes: 8192,
        }
    }
}

impl<'a> MakeWriter<'a> for TerminalLogBuffer {
    type Writer = TerminalLogWriter;
    fn make_writer(&'a self) -> Self::Writer {
        TerminalLogWriter::new(self.clone())
    }
}
