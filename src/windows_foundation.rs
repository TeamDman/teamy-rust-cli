//! Injectable tray and console state; no native listener is started here.
use std::io;

/// The caller must establish ownership; inherited and `ConPTY` hosts are shared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleState {
    OwnedVisible,
    OwnedHidden,
    SharedOrConPty,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    Show,
    Hide,
    Exit,
}

/// Native behavior is supplied by the application, on its message-loop owner.
pub trait DesktopBackend {
    /// # Errors
    /// Returns an error if creating the tray entry fails.
    fn create_tray(&mut self) -> io::Result<()>;
    /// # Errors
    /// Returns an error if removing the tray entry fails.
    fn remove_tray(&mut self) -> io::Result<()>;
    /// # Errors
    /// Returns an error if changing the owned console fails.
    fn set_owned_console_visible(&mut self, visible: bool) -> io::Result<()>;
}

/// A caller-driven state adapter, not a message loop or a native backend.
#[derive(Debug)]
pub struct DesktopSession<B> {
    backend: B,
    console: ConsoleState,
    tray_active: bool,
}

impl<B: DesktopBackend> DesktopSession<B> {
    #[must_use]
    pub fn new(backend: B, console: ConsoleState) -> Self {
        Self {
            backend,
            console,
            tray_active: false,
        }
    }
    #[must_use]
    pub fn console(&self) -> ConsoleState {
        self.console
    }
    #[must_use]
    pub fn tray_active(&self) -> bool {
        self.tray_active
    }
    /// Explicitly create a tray entry.
    /// # Errors
    /// Returns the backend failure without changing the state.
    pub fn activate(&mut self) -> io::Result<()> {
        if !self.tray_active {
            self.backend.create_tray()?;
            self.tray_active = true;
        }
        Ok(())
    }
    /// Apply a tray action; `true` requests application exit.
    /// # Errors
    /// Returns an error for shared or unavailable consoles or backend failure.
    pub fn handle(&mut self, action: TrayAction) -> io::Result<bool> {
        let visible = match action {
            TrayAction::Exit => return Ok(true),
            TrayAction::Show => true,
            TrayAction::Hide => false,
        };
        let next = match (self.console, visible) {
            (ConsoleState::OwnedVisible, true) | (ConsoleState::OwnedHidden, false) => {
                return Ok(false);
            }
            (ConsoleState::OwnedVisible, false) => ConsoleState::OwnedHidden,
            (ConsoleState::OwnedHidden, true) => ConsoleState::OwnedVisible,
            (ConsoleState::SharedOrConPty | ConsoleState::Unavailable, _) => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "console is not app-owned",
                ));
            }
        };
        self.backend.set_owned_console_visible(visible)?;
        self.console = next;
        Ok(false)
    }
    /// Restore an owned hidden console and remove the tray entry. Retry after failure.
    /// # Errors
    /// Returns a restoration or removal failure.
    pub fn shutdown(&mut self) -> io::Result<()> {
        let restore = if self.console == ConsoleState::OwnedHidden {
            self.handle(TrayAction::Show).map(|_| ())
        } else {
            Ok(())
        };
        let remove = if self.tray_active {
            self.backend.remove_tray().map(|()| {
                self.tray_active = false;
            })
        } else {
            Ok(())
        };
        restore.and(remove)
    }
}
