//! Private bounded owner/UI-child protocol and retained in-memory history.
mod child_channel;
mod history;
mod launcher;
mod protocol;
mod supervisor;

pub use child_channel::connect_child;
pub use history::Entry;
pub use history::History;
pub use history::HistoryStats;
pub use launcher::ChildConsole;
pub use protocol::MAX_APP_BYTES;
pub use protocol::MAX_FRAME_BYTES;
pub use protocol::Message;
pub use protocol::read_message;
pub use protocol::read_next;
pub use protocol::write_message;
pub use supervisor::ChildEvent;
pub use supervisor::Supervisor;
