#![cfg(feature = "inline-terminal")]

use std::io::Write;
use std::io::{self};
use teamy_rust_cli::terminal::InlineTerminal;
use teamy_rust_cli::terminal::ScreenStack;
use teamy_rust_cli::terminal::TerminalLogBuffer;
use teamy_rust_cli::terminal::print_above;
use tracing_subscriber::fmt::MakeWriter;

fn record(buffer: &TerminalLogBuffer, text: &str) {
    let mut writer = buffer.make_writer();
    writer.write_all(text.as_bytes()).unwrap();
}

#[test]
fn bounded_records_report_eviction_and_truncation() {
    let buffer = TerminalLogBuffer::new(2, 4).unwrap();
    record(&buffer, "first");
    record(&buffer, "second");
    record(&buffer, "ok");
    let snapshot = buffer.snapshot(10).unwrap();
    assert_eq!((snapshot.evicted, snapshot.truncated), (1, 2));
    assert_eq!(snapshot.entries.len(), 2);
    assert_eq!(snapshot.entries[0].text, "seco");
    assert!(snapshot.entries[0].truncated);
    buffer.acknowledge(snapshot.entries[0].sequence).unwrap();
    assert_eq!(buffer.snapshot(10).unwrap().entries[0].text, "ok");
}

struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic failure"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn replay_acknowledges_only_after_success() {
    let buffer = TerminalLogBuffer::default();
    record(&buffer, "one");
    record(&buffer, "two");
    assert!(buffer.replay(Broken).is_err());
    assert_eq!(buffer.snapshot(10).unwrap().entries.len(), 2);
    let mut output = Vec::new();
    buffer.replay(&mut output).unwrap();
    assert_eq!(output, b"onetwo");
    assert!(buffer.snapshot(10).unwrap().entries.is_empty());
}

#[test]
fn caller_can_layer_terminal_capture_with_an_independent_sink() {
    use tracing_subscriber::prelude::*;
    let terminal = TerminalLogBuffer::default();
    let independent = TerminalLogBuffer::default();
    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_ansi(false)
                .with_writer(terminal.clone()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_ansi(false)
                .with_writer(independent.clone()),
        );
    tracing::subscriber::with_default(subscriber, || tracing::info!("synthetic event"));
    assert!(
        terminal.snapshot(1).unwrap().entries[0]
            .text
            .contains("synthetic event")
    );
    assert!(
        independent.snapshot(1).unwrap().entries[0]
            .text
            .contains("synthetic event")
    );
    terminal.replay(Vec::new()).unwrap();
    assert_eq!(independent.snapshot(10).unwrap().entries.len(), 1);
}

#[test]
fn screen_stack_is_bounded_and_keeps_root() {
    let mut screens = ScreenStack::new("overview", 2).unwrap();
    screens.push("detail").unwrap();
    assert_eq!(screens.push("extra"), Err("extra"));
    assert_eq!(screens.pop(), Some("detail"));
    assert_eq!(screens.pop(), None);
    assert_eq!(screens.current(), Some(&"overview"));
    InlineTerminal::open(io::sink(), 0).unwrap_err();
}

#[test]
fn untrusted_log_is_escaped_above_inline_view() {
    use ratatui::Terminal;
    use ratatui::TerminalOptions;
    use ratatui::Viewport;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Paragraph;
    let mut terminal = Terminal::with_options(
        TestBackend::new(48, 8),
        TerminalOptions {
            viewport: Viewport::Inline(3),
        },
    )
    .unwrap();
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new("overview"), frame.area()))
        .unwrap();
    print_above(&mut terminal, "log\u{1b}[31m").unwrap();
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new("overview"), frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let visible = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(visible.contains("log\\u{1b}[31m"));
    assert!(visible.contains("overview"));
}
