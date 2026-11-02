//! Synthetic PTY probe; does not read or send user data.
use std::io::IsTerminal;
use std::io::Write;
use std::io::{self};
use teamy_rust_cli::terminal::InlineTerminal;
use teamy_rust_cli::terminal::TerminalLogBuffer;
use teamy_rust_cli::terminal::print_above;
use tracing_subscriber::fmt::MakeWriter;

struct FailingWriter {
    fail: std::rc::Rc<std::cell::Cell<bool>>,
}
impl Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.fail.get() {
            Err(io::Error::other("synthetic writer failure"))
        } else {
            io::stderr().write(buf)
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fail.get() {
            Err(io::Error::other("synthetic writer failure"))
        } else {
            io::stderr().flush()
        }
    }
}

fn failed_restoration() -> io::Result<()> {
    let fail = std::rc::Rc::new(std::cell::Cell::new(false));
    let mut terminal = InlineTerminal::open(
        FailingWriter {
            fail: std::rc::Rc::clone(&fail),
        },
        3,
    )?;
    fail.set(true);
    terminal.close().unwrap_err();
    assert!(!crossterm::terminal::is_raw_mode_enabled()?);
    assert_eq!(
        InlineTerminal::open(io::stderr(), 3).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    terminal.close().unwrap_err();
    writeln!(
        io::stderr(),
        "SYNTHETIC PTY OK: failed restore rejects another owner"
    )
}

fn session(fail: bool) -> io::Result<()> {
    let mut terminal = InlineTerminal::open(io::stderr(), 4)?;
    let nested = InlineTerminal::open(io::stderr(), 4).unwrap_err();
    assert_eq!(nested.kind(), io::ErrorKind::AlreadyExists);
    let capture = TerminalLogBuffer::new(2, 32)?;
    for line in ["one", "two", "three"] {
        write!(capture.make_writer(), "{line}")?;
    }
    let snapshot = capture.snapshot(10)?;
    assert_eq!((snapshot.entries.len(), snapshot.evicted), (2, 1));
    let active = terminal
        .terminal_mut()
        .ok_or_else(|| io::Error::other("closed"))?;
    print_above(active, "synthetic \u{1b}[31m")?;
    for record in snapshot.entries {
        print_above(active, &record.text)?;
        capture.acknowledge(record.sequence)?;
    }
    assert!(capture.snapshot(10)?.entries.is_empty());
    active.draw(|frame| {
        frame.render_widget(
            ratatui::widgets::Paragraph::new("synthetic inline view"),
            frame.area(),
        );
    })?;
    if fail {
        return Err(io::Error::other("synthetic application error"));
    }
    terminal.close()
}

fn main() -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(io::Error::other("probe requires a synthetic PTY"));
    }
    assert!(!crossterm::terminal::is_raw_mode_enabled()?);
    let mode = std::env::args().nth(1);
    if mode.as_deref() == Some("close-error") {
        return failed_restoration();
    }
    let fail = mode.as_deref() == Some("error");
    let result = session(fail);
    if fail {
        assert_eq!(
            result.unwrap_err().to_string(),
            "synthetic application error"
        );
    } else {
        result?;
    }
    assert!(!crossterm::terminal::is_raw_mode_enabled()?);
    let mut again = InlineTerminal::open(io::stderr(), 2)?;
    again.close()?;
    assert!(!crossterm::terminal::is_raw_mode_enabled()?);
    writeln!(
        io::stderr(),
        "SYNTHETIC PTY OK: nested rejected, restored, reopened"
    )
}
