use super::super::TrayAction;
use std::cell::RefCell;
use std::io;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::thread::{self};
use std::time::Duration;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::POINT;
use windows::Win32::Foundation::WAIT_FAILED;
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateEventW;
use windows::Win32::System::Threading::SetEvent;
use windows::Win32::UI::Shell::NIF_ICON;
use windows::Win32::UI::Shell::NIF_MESSAGE;
use windows::Win32::UI::Shell::NIF_TIP;
use windows::Win32::UI::Shell::NIM_ADD;
use windows::Win32::UI::Shell::NIM_DELETE;
use windows::Win32::UI::Shell::NOTIFYICONDATAW;
use windows::Win32::UI::Shell::Shell_NotifyIconW;
use windows::Win32::UI::WindowsAndMessaging::AppendMenuW;
use windows::Win32::UI::WindowsAndMessaging::CreatePopupMenu;
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::DestroyMenu;
use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::Win32::UI::WindowsAndMessaging::IDI_APPLICATION;
use windows::Win32::UI::WindowsAndMessaging::LoadIconW;
use windows::Win32::UI::WindowsAndMessaging::MF_STRING;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::MWMO_INPUTAVAILABLE;
use windows::Win32::UI::WindowsAndMessaging::MsgWaitForMultipleObjectsEx;
use windows::Win32::UI::WindowsAndMessaging::PM_REMOVE;
use windows::Win32::UI::WindowsAndMessaging::PeekMessageW;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::QS_ALLINPUT;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassW;
use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::TPM_RETURNCMD;
use windows::Win32::UI::WindowsAndMessaging::TPM_RIGHTBUTTON;
use windows::Win32::UI::WindowsAndMessaging::TrackPopupMenu;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSW;
use windows::Win32::UI::WindowsAndMessaging::WS_OVERLAPPED;
use windows::core::w;

const CALLBACK: u32 = WM_APP + 24;
const SHOW: usize = 1;
const HIDE: usize = 2;
const EXIT: usize = 3;

/// Typed user actions and asynchronous native failures. No command strings are accepted.
#[derive(Debug)]
pub enum TrayEvent {
    Action(TrayAction),
    Error(String),
}

/// Bounded, nonblocking event delivery. A full or disconnected queue increments loss.
#[derive(Debug)]
pub struct TrayEvents {
    receiver: mpsc::Receiver<TrayEvent>,
    dropped: Arc<AtomicU64>,
}
impl TrayEvents {
    /// Receive one event without blocking.
    /// # Errors
    /// Returns an error when no event is ready or all senders have disconnected.
    pub fn try_recv(&self) -> Result<TrayEvent, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug)]
pub(super) struct EventSink {
    sender: mpsc::SyncSender<TrayEvent>,
    dropped: Arc<AtomicU64>,
}
impl EventSink {
    fn send(&self, event: TrayEvent) {
        if self.sender.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}
pub(super) fn channel(capacity: usize) -> io::Result<(EventSink, TrayEvents)> {
    if capacity == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "tray event capacity must be nonzero",
        ));
    }
    let (sender, receiver) = mpsc::sync_channel(capacity);
    let dropped = Arc::new(AtomicU64::new(0));
    Ok((
        EventSink {
            sender,
            dropped: Arc::clone(&dropped),
        },
        TrayEvents { receiver, dropped },
    ))
}

struct State {
    events: EventSink,
    taskbar_created: u32,
    cleanup_error: Option<String>,
}

/// A process-owned kernel event shared by the caller and message thread.
#[derive(Debug)]
struct ShutdownEvent(isize);
impl ShutdownEvent {
    fn new() -> io::Result<Self> {
        // SAFETY: No security attributes or name are supplied; the returned handle is owned here.
        let handle = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
        Ok(Self(handle.0 as isize))
    }
    fn handle(&self) -> HANDLE {
        HANDLE(self.0 as *mut std::ffi::c_void)
    }
    fn signal(&self) -> io::Result<()> {
        // SAFETY: The Arc-shared handle remains alive throughout the call.
        unsafe { SetEvent(self.handle()) }.map_err(io::Error::other)
    }
}
impl Drop for ShutdownEvent {
    fn drop(&mut self) {
        // SAFETY: The last Arc is gone; this is the unique close of the owned event handle.
        let _ = unsafe { CloseHandle(self.handle()) };
    }
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
static CLASS: OnceLock<Result<(), String>> = OnceLock::new();

fn emit(event: TrayEvent) {
    STATE.with(|cell| {
        if let Ok(state) = cell.try_borrow()
            && let Some(state) = state.as_ref()
        {
            state.events.send(event);
        }
    });
}

fn icon(hwnd: HWND, add: bool) -> io::Result<()> {
    let mut data = NOTIFYICONDATAW {
        cbSize: u32::try_from(std::mem::size_of::<NOTIFYICONDATAW>()).map_err(io::Error::other)?,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    if add {
        // SAFETY: IDI_APPLICATION denotes a shared system icon; no ownership transfer occurs.
        data.hIcon = unsafe { LoadIconW(None, IDI_APPLICATION) }.map_err(io::Error::other)?;
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        data.uCallbackMessage = CALLBACK;
        for (target, value) in data.szTip.iter_mut().zip("Teamy".encode_utf16()) {
            *target = value;
        }
    }
    // SAFETY: data is initialized, remains live during the call, and hwnd belongs to this message thread.
    if !unsafe { Shell_NotifyIconW(if add { NIM_ADD } else { NIM_DELETE }, &raw const data) }
        .as_bool()
    {
        return Err(io::Error::other(if add {
            "Shell_NotifyIconW add failed"
        } else {
            "Shell_NotifyIconW delete failed"
        }));
    }
    Ok(())
}

fn restore_icon(mut add: impl FnMut() -> io::Result<()>, events: &EventSink) {
    if let Err(error) = add() {
        events.send(TrayEvent::Error(format!("restore tray icon: {error}")));
    }
}

fn menu(hwnd: HWND) -> io::Result<Option<TrayAction>> {
    // SAFETY: The menu is owned by this thread and destroyed before returning.
    let menu = unsafe { CreatePopupMenu() }.map_err(io::Error::other)?;
    let result = (|| {
        // SAFETY: menu and static UTF-16 strings are valid for these calls.
        unsafe { AppendMenuW(menu, MF_STRING, SHOW, w!("Show console")) }
            .map_err(io::Error::other)?;
        // SAFETY: Same menu and string lifetime as above.
        unsafe { AppendMenuW(menu, MF_STRING, HIDE, w!("Hide console")) }
            .map_err(io::Error::other)?;
        // SAFETY: Same menu and string lifetime as above.
        unsafe { AppendMenuW(menu, MF_STRING, EXIT, w!("Exit")) }.map_err(io::Error::other)?;
        let mut point = POINT::default();
        // SAFETY: point is valid writable memory.
        unsafe { GetCursorPos(&raw mut point) }.map_err(io::Error::other)?;
        // SAFETY: hwnd is this thread's hidden top-level window.
        let _ = unsafe { SetForegroundWindow(hwnd) };
        // SAFETY: menu remains live for the duration of the modal menu operation.
        let selected = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                point.x,
                point.y,
                None,
                hwnd,
                None,
            )
        }
        .0;
        Ok(match usize::try_from(selected).ok() {
            Some(SHOW) => Some(TrayAction::Show),
            Some(HIDE) => Some(TrayAction::Hide),
            Some(EXIT) => Some(TrayAction::Exit),
            _ => None,
        })
    })();
    // SAFETY: The menu was created by this function and is no longer in use.
    let destroy = unsafe { DestroyMenu(menu) }.map_err(io::Error::other);
    match (result, destroy) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(action), Ok(())) => Ok(action),
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let restart = STATE.with(|cell| {
        cell.try_borrow()
            .ok()
            .and_then(|state| state.as_ref().map(|state| state.taskbar_created))
    });
    if restart == Some(message) && message != 0 {
        let events = STATE.with(|cell| {
            cell.try_borrow()
                .ok()
                .and_then(|state| state.as_ref().map(|state| state.events.clone()))
        });
        if let Some(events) = events {
            restore_icon(|| icon(hwnd, true), &events);
        }
        return LRESULT(0);
    }
    match message {
        CALLBACK if u32::try_from(lparam.0).ok() == Some(WM_RBUTTONUP) => {
            match menu(hwnd) {
                Ok(Some(action)) => emit(TrayEvent::Action(action)),
                Ok(None) => {}
                Err(error) => emit(TrayEvent::Error(format!("tray menu: {error}"))),
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            close_window(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: Posts WM_QUIT to this message thread only.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: Unhandled window messages are delegated with the original parameters.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn close_window(hwnd: HWND) {
    if let Err(error) = icon(hwnd, false) {
        let message = format!("remove tray icon: {error}");
        emit(TrayEvent::Error(message.clone()));
        STATE.with(|cell| {
            if let Ok(mut state) = cell.try_borrow_mut()
                && let Some(state) = state.as_mut()
            {
                state.cleanup_error = Some(message);
            }
        });
    }
    // SAFETY: hwnd was created on this message thread and is being closed here.
    if let Err(error) = unsafe { DestroyWindow(hwnd) } {
        let message = format!("destroy tray window: {error}");
        emit(TrayEvent::Error(message.clone()));
        STATE.with(|cell| {
            if let Ok(mut state) = cell.try_borrow_mut()
                && let Some(state) = state.as_mut()
            {
                state.cleanup_error = Some(message);
            }
        });
    }
}

fn message_thread(
    events: EventSink,
    ready: &mpsc::SyncSender<Result<(), String>>,
    shutdown: &ShutdownEvent,
) -> io::Result<()> {
    // SAFETY: A null module name selects the current executable module.
    let instance = unsafe { GetModuleHandleW(None) }.map_err(io::Error::other)?;
    let registered = CLASS.get_or_init(|| {
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: w!("TeamyRustCliTray"),
            ..Default::default()
        };
        // SAFETY: class and its static name are valid for the duration of registration.
        if unsafe { RegisterClassW(&raw const class) } == 0 {
            Err(io::Error::last_os_error().to_string())
        } else {
            Ok(())
        }
    });
    registered
        .as_ref()
        .map_err(|error| io::Error::other(error.clone()))?;
    // SAFETY: The message name is a valid static UTF-16 string.
    let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    if taskbar_created == 0 {
        return Err(io::Error::last_os_error());
    }
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(State {
            events,
            taskbar_created,
            cleanup_error: None,
        });
    });
    // SAFETY: The registered class, instance and static title are valid; no parent, menu or user pointer is passed.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("TeamyRustCliTray"),
            w!("Teamy"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(io::Error::other)?;
    if let Err(error) = icon(hwnd, true) {
        // SAFETY: Window creation succeeded on this thread and no icon was added.
        let _ = unsafe { DestroyWindow(hwnd) };
        return Err(error);
    }
    if ready.send(Ok(())).is_err() {
        let _ = icon(hwnd, false);
        // SAFETY: Window creation succeeded on this thread; startup receiver disappeared.
        let _ = unsafe { DestroyWindow(hwnd) };
        return Err(io::Error::other("tray startup receiver disconnected"));
    }
    let handles = [shutdown.handle()];
    'messages: loop {
        // SAFETY: The Arc-owned event remains valid and the slice lives through the wait.
        let result = unsafe {
            MsgWaitForMultipleObjectsEx(Some(&handles), u32::MAX, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
        };
        if result == WAIT_OBJECT_0 {
            close_window(hwnd);
            break;
        }
        if result == WAIT_FAILED {
            let error = io::Error::last_os_error();
            close_window(hwnd);
            return Err(io::Error::other(format!(
                "tray message wait failed: {error}"
            )));
        }
        if result.0 != WAIT_OBJECT_0.0 + 1 {
            close_window(hwnd);
            return Err(io::Error::other(format!(
                "unexpected tray wait result: {}",
                result.0
            )));
        }
        let mut message = MSG::default();
        // Limit each batch so a busy message queue cannot indefinitely delay shutdown.
        for _ in 0..64 {
            // SAFETY: message is writable and only messages for this thread are removed.
            if !unsafe { PeekMessageW(&raw mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                break;
            }
            if message.message == WM_QUIT {
                break 'messages;
            }
            // SAFETY: PeekMessageW populated a valid thread message.
            let _ = unsafe { TranslateMessage(&raw const message) };
            // SAFETY: The message remains valid for dispatch.
            unsafe { DispatchMessageW(&raw const message) };
        }
    }
    let error = STATE.with(|cell| {
        cell.borrow_mut()
            .take()
            .and_then(|state| state.cleanup_error)
    });
    error.map_or(Ok(()), |error| Err(io::Error::other(error)))
}

#[derive(Debug)]
pub(super) struct TrayHandle {
    shutdown: Arc<ShutdownEvent>,
    finished: mpsc::Receiver<io::Result<()>>,
    thread: Option<JoinHandle<()>>,
}
impl TrayHandle {
    pub(super) fn finished(&self) -> bool {
        self.thread.is_none()
    }
    fn finish(&mut self, result: io::Result<()>) -> io::Result<()> {
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_panic| io::Error::other("tray thread panicked"))?;
        }
        result
    }
    pub(super) fn start(events: EventSink) -> io::Result<Self> {
        let shutdown = Arc::new(ShutdownEvent::new()?);
        let thread_shutdown = Arc::clone(&shutdown);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("teamy-tray".into())
            .spawn(move || {
                let result = message_thread(events, &ready_tx, &thread_shutdown);
                if let Err(error) = &result {
                    let _ = ready_tx.try_send(Err(error.to_string()));
                }
                let _ = done_tx.send(result);
            })?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                shutdown,
                finished: done_rx,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(io::Error::other(error))
            }
            Err(_) => {
                let _ = thread.join();
                Err(io::Error::other("tray startup thread stopped"))
            }
        }
    }
    pub(super) fn shutdown(&mut self) -> io::Result<()> {
        match self.finished.try_recv() {
            Ok(result) => return self.finish(result),
            Err(mpsc::TryRecvError::Disconnected) => {
                return self.finish(Err(io::Error::other("tray completion disconnected")));
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        self.shutdown.signal()?;
        match self.finished.recv_timeout(Duration::from_secs(2)) {
            Ok(result) => self.finish(result),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "tray shutdown pending; retry",
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.finish(Err(io::Error::other("tray completion disconnected")))
            }
        }
    }
}

impl Drop for TrayHandle {
    fn drop(&mut self) {
        if self.thread.is_some() {
            let _ = self.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_events_report_loss_and_restart_errors() {
        let (sink, events) = channel(1).unwrap();
        sink.send(TrayEvent::Action(TrayAction::Show));
        sink.send(TrayEvent::Action(TrayAction::Hide));
        assert_eq!(events.dropped(), 1);
        assert!(matches!(
            events.try_recv().unwrap(),
            TrayEvent::Action(TrayAction::Show)
        ));
        let mut calls = 0;
        restore_icon(
            || {
                calls += 1;
                Err(io::Error::other("synthetic shell failure"))
            },
            &sink,
        );
        assert_eq!(calls, 1);
        assert!(
            matches!(events.try_recv().unwrap(), TrayEvent::Error(message) if message.contains("synthetic shell failure"))
        );
    }
}
