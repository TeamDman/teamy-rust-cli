use super::escape_terminal_text;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::io::Write;
use std::io::{self};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use unicode_width::UnicodeWidthChar;

static TERMINAL_OWNED: AtomicBool = AtomicBool::new(false);

/// Insert one escaped, width-limited line above an inline viewport.
/// # Errors
/// Returns the terminal backend error when insertion fails.
pub fn print_above<B: Backend>(terminal: &mut Terminal<B>, text: &str) -> Result<(), B::Error> {
    let width = usize::from(terminal.size()?.width);
    let text = escape_terminal_text(text)
        .replace('\n', " ⏎ ")
        .replace('\t', " ");
    let mut line = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cells = ch.width().unwrap_or(0);
        if used.saturating_add(cells) >= width {
            if width > 0 {
                line.push('…');
            }
            break;
        }
        used += cells;
        line.push(ch);
    }
    terminal.insert_before(1, |buffer| Paragraph::new(line).render(buffer.area, buffer))
}

/// Owns raw mode and an inline viewport using a caller-provided writer.
#[derive(Debug)]
pub struct InlineTerminal<W: Write> {
    terminal: Option<Terminal<CrosstermBackend<W>>>,
    raw_active: bool,
    owned: bool,
    screen_failed: bool,
}

impl<W: Write> InlineTerminal<W> {
    /// The caller is responsible for checking terminal ownership and interactivity.
    /// # Errors
    /// Returns an error if the height, raw-mode transition or terminal setup fails.
    pub fn open(writer: W, height: u16) -> io::Result<Self> {
        if height == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "inline height must be nonzero",
            ));
        }
        if TERMINAL_OWNED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "inline terminal already owned or restoration failed",
            ));
        }
        if let Err(error) = enable_raw_mode() {
            TERMINAL_OWNED.store(false, Ordering::Release);
            return Err(error);
        }
        let terminal = Terminal::with_options(
            CrosstermBackend::new(writer),
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        );
        match terminal {
            Ok(terminal) => Ok(Self {
                terminal: Some(terminal),
                raw_active: true,
                owned: true,
                screen_failed: false,
            }),
            Err(error) => match disable_raw_mode() {
                Ok(()) => {
                    TERMINAL_OWNED.store(false, Ordering::Release);
                    Err(error)
                }
                Err(restore) => Err(io::Error::other(format!(
                    "terminal setup failed: {error}; raw-mode restoration failed: {restore}"
                ))),
            },
        }
    }
    pub fn terminal_mut(&mut self) -> Option<&mut Terminal<CrosstermBackend<W>>> {
        self.terminal.as_mut()
    }

    /// Attempt all restoration steps, returning the first failure.
    /// # Errors
    /// Returns an error if clearing, cursor restoration, flushing or raw-mode restoration fails.
    pub fn close(&mut self) -> io::Result<()> {
        if !self.owned {
            return Ok(());
        }
        let screen = if let Some(mut terminal) = self.terminal.take() {
            let origin = terminal.get_frame().area().as_position();
            let clear = terminal.clear();
            let position = terminal.set_cursor_position(origin);
            let cursor = terminal.show_cursor();
            let flush = Write::flush(terminal.backend_mut());
            clear.and(position).and(cursor).and(flush)
        } else {
            Ok(())
        };
        self.screen_failed |= screen.is_err();
        let raw = if self.raw_active {
            let result = disable_raw_mode();
            if result.is_ok() {
                self.raw_active = false;
            }
            result
        } else {
            Ok(())
        };
        if !self.screen_failed && raw.is_ok() {
            self.owned = false;
            TERMINAL_OWNED.store(false, Ordering::Release);
        }
        if self.screen_failed && screen.is_ok() {
            return Err(io::Error::other(
                "terminal screen restoration previously failed",
            ));
        }
        screen.and(raw)
    }
}

impl<W: Write> Drop for InlineTerminal<W> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
