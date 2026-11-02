//! Optional inline-terminal building blocks; callers own subscriber and sink setup.
mod inline_terminal;
mod log_buffer;
mod log_entry;
mod log_snapshot;
mod log_writer;
mod screen_stack;
mod text;

pub use inline_terminal::InlineTerminal;
pub use inline_terminal::print_above;
pub use log_buffer::TerminalLogBuffer;
pub use log_entry::TerminalLogEntry;
pub use log_snapshot::TerminalLogSnapshot;
pub use log_writer::TerminalLogWriter;
pub use screen_stack::ScreenStack;
pub use text::escape_terminal_text;
