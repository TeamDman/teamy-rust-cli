//! Windows launcher with non-inheritable anonymous pipes and verified-parent duplication.
use super::ChildConsole;
use super::ExitStatus;
use super::OsString;
use super::Path;
use super::Spawned;
use super::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::io::FromRawHandle;
use std::os::windows::io::OwnedHandle;
use std::os::windows::process::ExitStatusExt;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::Foundation::WAIT_TIMEOUT;
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::CREATE_NEW_CONSOLE;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows::Win32::System::Threading::CreateProcessW;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::System::Threading::GetExitCodeProcess;
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::Threading::STARTUPINFOW;
use windows::Win32::System::Threading::TerminateProcess;
use windows::Win32::System::Threading::WaitForSingleObject;
use windows::core::PCWSTR;
use windows::core::PWSTR;

#[derive(Debug)]
pub(in crate::ui_process) struct Process {
    handle: OwnedHandle,
}
impl Process {
    fn raw(&self) -> HANDLE {
        HANDLE(self.handle.as_raw_handle())
    }
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        // SAFETY: The process handle is owned and remains valid for the call.
        match unsafe { WaitForSingleObject(self.raw(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0u32;
                // SAFETY: code is writable and the process handle is valid.
                unsafe { GetExitCodeProcess(self.raw(), &raw mut code) }
                    .map_err(io::Error::other)?;
                Ok(Some(ExitStatus::from_raw(code)))
            }
            _ => Err(io::Error::last_os_error()),
        }
    }
    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: This handle refers only to the child this launcher created.
        unsafe { TerminateProcess(self.raw(), 1) }.map_err(io::Error::other)
    }
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: The process handle remains owned while waiting for this child.
        if unsafe { WaitForSingleObject(self.raw(), u32::MAX) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        self.try_wait()?
            .ok_or_else(|| io::Error::other("child did not exit"))
    }
}

/// Parent copies of child ends are held only until Ready or child exit.
#[derive(Debug)]
pub(in crate::ui_process) struct Bootstrap {
    _input: OwnedHandle,
    _output: OwnedHandle,
}

fn pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let (mut read, mut write) = (HANDLE::default(), HANDLE::default());
    // SAFETY: Both output pointers are valid. Null attributes create non-inheritable handles.
    unsafe { CreatePipe(&raw mut read, &raw mut write, None, 0) }.map_err(io::Error::other)?;
    // SAFETY: CreatePipe returned two distinct owned handles, transferred once each.
    let read = unsafe { OwnedHandle::from_raw_handle(read.0) };
    // SAFETY: The write handle has separate ownership.
    let write = unsafe { OwnedHandle::from_raw_handle(write.0) };
    Ok((read, write))
}

fn quote(value: &std::ffi::OsStr) -> io::Result<Vec<u16>> {
    let mut result = vec![u16::from(b'"')];
    let mut slashes = 0usize;
    for unit in value.encode_wide() {
        if unit == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "argument contains NUL",
            ));
        }
        if unit == u16::from(b'\\') {
            slashes += 1;
            continue;
        }
        let count = if unit == u16::from(b'"') {
            slashes * 2 + 1
        } else {
            slashes
        };
        result.extend(std::iter::repeat_n(u16::from(b'\\'), count));
        result.push(unit);
        slashes = 0;
    }
    result.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2));
    result.push(u16::from(b'"'));
    Ok(result)
}

pub(in crate::ui_process) fn spawn(
    program: &Path,
    args: &[OsString],
    console: ChildConsole,
) -> io::Result<Spawned> {
    let (child_input, parent_input) = pipe()?;
    let (parent_output, child_output) = pipe()?;
    // SAFETY: This function only reads the current process identifier.
    let parent = unsafe { GetCurrentProcessId() };
    // SAFETY: This function obtains the current process pseudo-handle.
    let current = unsafe { GetCurrentProcess() };
    let creation = crate::ui_process::child_channel::creation_time(current)?;
    let bootstrap = [
        OsString::from("--teamy-ui-ipc"),
        OsString::from(parent.to_string()),
        OsString::from(creation.to_string()),
        OsString::from((child_input.as_raw_handle() as usize).to_string()),
        OsString::from((child_output.as_raw_handle() as usize).to_string()),
    ];
    let mut command = quote(program.as_os_str())?;
    for arg in args.iter().chain(&bootstrap) {
        command.push(u16::from(b' '));
        command.extend(quote(arg)?);
    }
    command.push(0);
    let mut executable = program.as_os_str().encode_wide().collect::<Vec<_>>();
    if executable.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "program contains NUL",
        ));
    }
    executable.push(0);
    let startup = STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).map_err(io::Error::other)?,
        ..Default::default()
    };
    let mut info = PROCESS_INFORMATION::default();
    let flags = match console {
        ChildConsole::Hidden => CREATE_NO_WINDOW,
        ChildConsole::NewConsole => CREATE_NEW_CONSOLE,
    };
    // SAFETY: The executable, mutable command line and structures remain live; FALSE forbids all handle inheritance.
    unsafe {
        CreateProcessW(
            PCWSTR(executable.as_ptr()),
            Some(PWSTR(command.as_mut_ptr())),
            None,
            None,
            false,
            flags,
            None,
            PCWSTR::null(),
            &raw const startup,
            &raw mut info,
        )
    }
    .map_err(io::Error::other)?;
    // SAFETY: CreateProcessW returned distinct owned process and thread handles.
    let process = unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) };
    // SAFETY: Ownership of the separate thread handle is transferred once.
    let thread = unsafe { OwnedHandle::from_raw_handle(info.hThread.0) };
    drop(thread);
    Ok(Spawned {
        process: Process { handle: process },
        input: Box::new(std::fs::File::from(parent_input)),
        output: Box::new(std::fs::File::from(parent_output)),
        bootstrap: Some(Bootstrap {
            _input: child_input,
            _output: child_output,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::GetHandleInformation;
    use windows::Win32::Foundation::HANDLE_FLAG_INHERIT;

    #[test]
    fn created_pipe_ends_are_never_inheritable() {
        let (read, write) = pipe().unwrap();
        for handle in [&read, &write] {
            let mut flags = 0u32;
            // SAFETY: Each handle is a live pipe end created by this test.
            unsafe { GetHandleInformation(HANDLE(handle.as_raw_handle()), &raw mut flags) }
                .unwrap();
            assert_eq!(flags & HANDLE_FLAG_INHERIT.0, 0);
        }
    }
}
