use std::io;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Console::AllocConsole;
use windows::Win32::System::Console::FreeConsole;
use windows::Win32::System::Console::GetConsoleCP;
use windows::Win32::System::Console::GetConsoleWindow;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNA;
use windows::Win32::UI::WindowsAndMessaging::ShowWindow;

/// Proof of a console explicitly allocated by this process, not an inherited handle.
#[derive(Debug)]
pub struct OwnedConsole {
    hwnd: HWND,
}

impl OwnedConsole {
    /// Allocate a dedicated console only when the process has no existing console.
    /// This changes process standard handles and is never called automatically.
    /// # Errors
    /// Returns an error if any console is attached or allocation fails.
    pub fn allocate() -> io::Result<Self> {
        // SAFETY: These functions query process console state without dereferencing pointers.
        if unsafe { GetConsoleCP() } != 0 || !unsafe { GetConsoleWindow() }.is_invalid() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "process already has a console; refusing to replace it",
            ));
        }
        // SAFETY: The no-console check prevents detaching or manipulating an inherited console.
        unsafe { AllocConsole() }.map_err(io::Error::other)?;
        // SAFETY: Allocation just succeeded for this process.
        let hwnd = unsafe { GetConsoleWindow() };
        // SAFETY: IsWindow accepts a possibly invalid HWND and does not dereference Rust memory.
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            // SAFETY: This process just allocated the console; rollback detaches only that console.
            let rollback = unsafe { FreeConsole() };
            return Err(io::Error::other(format!(
                "allocated console has no usable window; rollback: {rollback:?}"
            )));
        }
        Ok(Self { hwnd })
    }

    fn validate(&self) -> io::Result<()> {
        // SAFETY: Both calls query the current process/window without dereferencing Rust pointers.
        let current = unsafe { GetConsoleWindow() };
        // SAFETY: IsWindow validates the stored handle without dereferencing Rust memory.
        let exists = unsafe { IsWindow(Some(self.hwnd)) }.as_bool();
        if current != self.hwnd || !exists {
            return Err(io::Error::other(
                "allocated console no longer belongs to this process",
            ));
        }
        Ok(())
    }

    pub(super) fn set_visible(&self, visible: bool) -> io::Result<()> {
        self.validate()?;
        // SAFETY: The token was created only after our successful allocation and revalidated above.
        let _ = unsafe { ShowWindow(self.hwnd, if visible { SW_SHOWNA } else { SW_HIDE }) };
        // SAFETY: The HWND remains valid unless another thread concurrently changes the console.
        if unsafe { IsWindowVisible(self.hwnd) }.as_bool() != visible {
            return Err(io::Error::other(
                "console visibility did not reach the requested state",
            ));
        }
        Ok(())
    }

    /// Detach this explicitly allocated console; never detaches an inherited one.
    /// # Errors
    /// Returns an error if the process console changed or detaching failed.
    pub fn release(&self) -> io::Result<()> {
        self.validate()?;
        // SAFETY: The private allocation token and current HWND establish the console we detach.
        unsafe { FreeConsole() }.map_err(io::Error::other)
    }
}
