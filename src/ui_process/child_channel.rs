use std::ffi::OsString;
use std::io::Read;
use std::io::Write;
use std::io::{self};

/// Open the dedicated owner pipes passed to a child created by this module.
/// On Windows, only the actual parent process may supply the handles.
/// # Errors
/// Returns an error for invalid bootstrap data, an unexpected parent or failed duplication.
#[cfg(windows)]
pub fn connect_child(
    args: &[OsString],
) -> io::Result<(Box<dyn Read + Send>, Box<dyn Write + Send>)> {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::io::FromRawHandle;
    use std::os::windows::io::OwnedHandle;
    use windows::Wdk::System::Threading::NtQueryInformationProcess;
    use windows::Wdk::System::Threading::ProcessBasicInformation;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Threading::GetCurrentProcess;
    use windows::Win32::System::Threading::OpenProcess;
    use windows::Win32::System::Threading::PROCESS_BASIC_INFORMATION;
    use windows::Win32::System::Threading::PROCESS_DUP_HANDLE;
    use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;

    let tail = args
        .get(
            args.len().checked_sub(5).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "missing UI bootstrap")
            })?..,
        )
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing UI bootstrap"))?;
    if tail[0] != "--teamy-ui-ipc" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing UI bootstrap marker",
        ));
    }
    let parse = |value: &OsString| -> io::Result<usize> {
        value
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid UI bootstrap"))?
            .parse()
            .map_err(|_error| {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid UI bootstrap number")
            })
    };
    let parent_id = parse(&tail[1])?;
    let creation = tail[2]
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid parent creation time"))?
        .parse::<u64>()
        .map_err(|_error| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid parent creation time")
        })?;
    let input = parse(&tail[3])?;
    let output = parse(&tail[4])?;
    let mut basic = PROCESS_BASIC_INFORMATION::default();
    // SAFETY: This function obtains the current process pseudo-handle.
    let current = unsafe { GetCurrentProcess() };
    // SAFETY: The pseudo-handle names this process and basic is a valid output buffer.
    let status = unsafe {
        NtQueryInformationProcess(
            current,
            ProcessBasicInformation,
            (&raw mut basic).cast(),
            u32::try_from(size_of::<PROCESS_BASIC_INFORMATION>()).map_err(io::Error::other)?,
            std::ptr::null_mut(),
        )
    };
    if status.0 < 0 {
        return Err(io::Error::other(format!(
            "query UI parent failed: {status:?}"
        )));
    }
    if basic.InheritedFromUniqueProcessId != parent_id {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "UI bootstrap parent does not match",
        ));
    }
    // SAFETY: The verified parent id is used only to obtain duplicate-handle access.
    let parent = unsafe {
        OpenProcess(
            PROCESS_DUP_HANDLE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            u32::try_from(parent_id).map_err(io::Error::other)?,
        )
    }
    .map_err(io::Error::other)?;
    // SAFETY: OpenProcess returned a handle owned by this child.
    let parent = unsafe { OwnedHandle::from_raw_handle(parent.0) };
    if creation_time(HANDLE(parent.as_raw_handle()))? != creation {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "UI bootstrap parent identity changed",
        ));
    }
    let input = duplicate(&parent, input)?;
    let output = duplicate(&parent, output)?;
    Ok((
        Box::new(std::fs::File::from(input)),
        Box::new(std::fs::File::from(output)),
    ))
}

#[cfg(windows)]
pub(in crate::ui_process) fn creation_time(
    process: windows::Win32::Foundation::HANDLE,
) -> io::Result<u64> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetProcessTimes;
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: The supplied process handle is valid and all four output pointers are writable.
    unsafe {
        GetProcessTimes(
            process,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        )
    }
    .map_err(io::Error::other)?;
    Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

#[cfg(windows)]
fn duplicate(
    parent: &std::os::windows::io::OwnedHandle,
    value: usize,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::io::FromRawHandle;
    use std::os::windows::io::OwnedHandle;
    use windows::Win32::Foundation::DUPLICATE_SAME_ACCESS;
    use windows::Win32::Foundation::DuplicateHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Threading::GetCurrentProcess;
    let mut result = HANDLE::default();
    // SAFETY: This function obtains the current process pseudo-handle.
    let current = unsafe { GetCurrentProcess() };
    // SAFETY: The source process is the verified parent; the output receives a new non-inheritable handle.
    unsafe {
        DuplicateHandle(
            HANDLE(parent.as_raw_handle()),
            HANDLE(value as *mut std::ffi::c_void),
            current,
            &raw mut result,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    }
    .map_err(io::Error::other)?;
    // SAFETY: DuplicateHandle returned a new owned handle, transferred exactly once.
    Ok(unsafe { OwnedHandle::from_raw_handle(result.0) })
}

/// Non-Windows fixtures use piped standard streams.
/// # Errors
/// Returns an error only if a future platform-specific connector cannot initialize.
#[cfg(not(windows))]
pub fn connect_child(
    _args: &[OsString],
) -> io::Result<(Box<dyn Read + Send>, Box<dyn Write + Send>)> {
    Ok((Box::new(io::stdin()), Box::new(io::stdout())))
}
