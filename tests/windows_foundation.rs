#![cfg(all(windows, feature = "windows-foundation"))]
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use teamy_rust_cli::windows_foundation::ConsoleState;
use teamy_rust_cli::windows_foundation::DesktopBackend;
use teamy_rust_cli::windows_foundation::DesktopSession;
use teamy_rust_cli::windows_foundation::TrayAction;

#[derive(Clone, Debug, Default)]
struct Fake(Rc<RefCell<Vec<&'static str>>>);
impl DesktopBackend for Fake {
    fn create_tray(&mut self) -> io::Result<()> {
        self.0.borrow_mut().push("create");
        Ok(())
    }
    fn remove_tray(&mut self) -> io::Result<()> {
        self.0.borrow_mut().push("remove");
        Ok(())
    }
    fn set_owned_console_visible(&mut self, visible: bool) -> io::Result<()> {
        self.0
            .borrow_mut()
            .push(if visible { "show" } else { "hide" });
        Ok(())
    }
}

#[test]
fn owned_console_is_restored_and_tray_is_removed() {
    let fake = Fake::default();
    let mut session = DesktopSession::new(fake.clone(), ConsoleState::OwnedVisible);
    session.activate().unwrap();
    assert!(!session.handle(TrayAction::Hide).unwrap());
    assert_eq!(session.console(), ConsoleState::OwnedHidden);
    assert!(session.handle(TrayAction::Exit).unwrap());
    session.shutdown().unwrap();
    assert_eq!(session.console(), ConsoleState::OwnedVisible);
    assert!(!session.tray_active());
    assert_eq!(*fake.0.borrow(), ["create", "hide", "show", "remove"]);
}

#[test]
fn shared_console_is_never_sent_to_backend() {
    let fake = Fake::default();
    let mut session = DesktopSession::new(fake.clone(), ConsoleState::SharedOrConPty);
    assert_eq!(
        session.handle(TrayAction::Hide).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        session.handle(TrayAction::Show).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(fake.0.borrow().is_empty());
}

#[derive(Debug)]
struct Broken;
impl DesktopBackend for Broken {
    fn create_tray(&mut self) -> io::Result<()> {
        Err(io::Error::other("create"))
    }
    fn remove_tray(&mut self) -> io::Result<()> {
        Err(io::Error::other("remove"))
    }
    fn set_owned_console_visible(&mut self, _: bool) -> io::Result<()> {
        Err(io::Error::other("visibility"))
    }
}

#[test]
fn failed_backend_actions_do_not_change_recorded_state() {
    let mut session = DesktopSession::new(Broken, ConsoleState::OwnedVisible);
    session.activate().unwrap_err();
    assert!(!session.tray_active());
    session.handle(TrayAction::Hide).unwrap_err();
    assert_eq!(session.console(), ConsoleState::OwnedVisible);
    session.shutdown().unwrap();
}
