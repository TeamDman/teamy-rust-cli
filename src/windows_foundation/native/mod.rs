//! Explicit Windows backend; constructing it does not activate a tray or console.
mod console;
mod tray;

use super::DesktopBackend;
pub use console::OwnedConsole;
use std::io;
pub use tray::TrayEvent;
pub use tray::TrayEvents;

/// Backend for a caller-owned desktop session. Its message loop starts on activation.
#[derive(Debug)]
pub struct NativeDesktopBackend {
    console: Option<OwnedConsole>,
    tray: Option<tray::TrayHandle>,
    events: tray::EventSink,
}

impl NativeDesktopBackend {
    /// Create an inactive backend and a bounded stream of typed tray events.
    /// # Errors
    /// Returns an error if the event capacity is zero.
    pub fn new(console: Option<OwnedConsole>, capacity: usize) -> io::Result<(Self, TrayEvents)> {
        let (events, receiver) = tray::channel(capacity)?;
        Ok((
            Self {
                console,
                tray: None,
                events,
            },
            receiver,
        ))
    }

    /// Detach a console that was explicitly allocated by this backend's caller.
    /// # Errors
    /// Returns an error if the console has changed or cannot be detached.
    pub fn release_console(&mut self) -> io::Result<()> {
        if let Some(console) = self.console.as_ref() {
            console.release()?;
            self.console = None;
        }
        Ok(())
    }
}

impl DesktopBackend for NativeDesktopBackend {
    fn create_tray(&mut self) -> io::Result<()> {
        if self.tray.is_none() {
            self.tray = Some(tray::TrayHandle::start(self.events.clone())?);
        }
        Ok(())
    }
    fn remove_tray(&mut self) -> io::Result<()> {
        if let Some(tray) = self.tray.as_mut() {
            let result = tray.shutdown();
            if tray.finished() {
                self.tray = None;
            }
            result?;
        }
        Ok(())
    }
    fn set_owned_console_visible(&mut self, visible: bool) -> io::Result<()> {
        self.console
            .as_ref()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "no allocated console token",
                )
            })?
            .set_visible(visible)
    }
}

impl Drop for NativeDesktopBackend {
    fn drop(&mut self) {
        if let Some(console) = &self.console {
            let _ = console.set_visible(true);
        }
        if let Some(tray) = &mut self.tray {
            let _ = tray.shutdown();
        }
    }
}
