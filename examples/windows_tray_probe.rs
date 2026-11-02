#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use std::io;
    use std::thread;
    use std::time::Duration;
    use teamy_rust_cli::windows_foundation::ConsoleState;
    use teamy_rust_cli::windows_foundation::DesktopSession;
    use teamy_rust_cli::windows_foundation::TrayAction;
    use teamy_rust_cli::windows_foundation::native::NativeDesktopBackend;
    use teamy_rust_cli::windows_foundation::native::OwnedConsole;
    use teamy_rust_cli::windows_foundation::native::TrayEvent;

    let (console, state) = match std::env::args().nth(1).as_deref() {
        Some("--shared") => (None, ConsoleState::SharedOrConPty),
        Some("--owned") => (Some(OwnedConsole::allocate()?), ConsoleState::OwnedVisible),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "select --shared or --owned",
            ));
        }
    };
    let (backend, events) = NativeDesktopBackend::new(console, 16)?;
    let mut session = DesktopSession::new(backend, state);
    session.activate()?;
    let result = loop {
        if events.dropped() != 0 {
            break Err(io::Error::other("tray event queue overflow"));
        }
        match events.try_recv() {
            Ok(TrayEvent::Action(TrayAction::Exit)) => break Ok(()),
            Ok(TrayEvent::Action(action)) if state == ConsoleState::SharedOrConPty => {
                if session.handle(action).unwrap_err().kind() != io::ErrorKind::PermissionDenied {
                    break Err(io::Error::other("shared console check failed"));
                }
            }
            Ok(TrayEvent::Action(action)) => {
                if let Err(error) = session.handle(action) {
                    break Err(error);
                }
            }
            Ok(TrayEvent::Error(error)) => break Err(io::Error::other(error)),
            Err(std::sync::mpsc::TryRecvError::Empty) => thread::sleep(Duration::from_millis(50)),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                break Err(io::Error::other("tray event channel disconnected"));
            }
        }
    };
    let shutdown = session.shutdown();
    let mut backend = session.into_backend();
    let release = backend.release_console();
    result.and(shutdown).and(release)
}

#[cfg(not(windows))]
fn main() {}
