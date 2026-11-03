use super::MAX_FRAME_BYTES;
use std::collections::VecDeque;
use std::io;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub sequence: u64,
    pub payload: Vec<u8>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HistoryStats {
    pub accepted: u64,
    pub evicted: u64,
    pub rejected: u64,
    pub records: usize,
    pub bytes: usize,
}

/// Retention belongs to the owner; transport acknowledgements never remove entries.
#[derive(Debug)]
pub struct History {
    records: VecDeque<Entry>,
    stats: HistoryStats,
    max_records: usize,
    max_bytes: usize,
}
impl History {
    /// # Errors
    /// Returns an error if a bound is zero.
    pub fn new(max_records: usize, max_bytes: usize) -> io::Result<Self> {
        if max_records == 0 || max_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "history bounds must be nonzero",
            ));
        }
        Ok(Self {
            records: VecDeque::new(),
            stats: HistoryStats::default(),
            max_records,
            max_bytes,
        })
    }
    /// Reject oversize records explicitly; evict oldest records to remain bounded.
    pub fn push(&mut self, payload: Vec<u8>) -> bool {
        if payload.len() > self.max_bytes
            || payload.len() > MAX_FRAME_BYTES - 18
            || self.stats.accepted == u64::MAX
        {
            self.stats.rejected = self.stats.rejected.saturating_add(1);
            return false;
        }
        while self.records.len() >= self.max_records
            || self.stats.bytes.saturating_add(payload.len()) > self.max_bytes
        {
            if let Some(old) = self.records.pop_front() {
                self.stats.bytes -= old.payload.len();
                self.stats.evicted = self.stats.evicted.saturating_add(1);
            }
        }
        self.stats.accepted += 1;
        self.stats.bytes += payload.len();
        self.records.push_back(Entry {
            sequence: self.stats.accepted,
            payload,
        });
        self.stats.records = self.records.len();
        true
    }
    #[must_use]
    pub fn stats(&self) -> HistoryStats {
        self.stats
    }
    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.records.iter()
    }
}
