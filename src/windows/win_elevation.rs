use crate::cli::to_args::Invocable;
use crate::cli::to_args::SameInvocationSameConsole;
use crate::windows::win_strings::EasyPCWSTR;
use eyre::bail;
use eyre::eyre;
use eyre::Context;
use tracing::info;
use tracing::warn;
use std::ffi::OsString;
use std::mem::size_of;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::GetTokenInformation;
use windows::Win32::Security::TOKEN_ELEVATION;
use windows::Win32::Security::TOKEN_QUERY;
use windows::Win32::Security::TokenElevation;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::GetExitCodeProcess;
use windows::Win32::System::Threading::INFINITE;
use windows::Win32::System::Threading::OpenProcessToken;
use windows::Win32::System::Threading::WaitForSingleObject;
use windows::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS;
use windows::Win32::UI::Shell::SHELLEXECUTEINFOW;
use windows::Win32::UI::Shell::ShellExecuteExW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Checks if the current process is running with elevated privileges.
pub fn is_elevated() -> bool {
    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_err() {
            eprintln!("Failed to open process token. Error: {:?}", GetLastError());
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0;

        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        if result.is_ok() {
            elevation.TokenIsElevated != 0
        } else {
            eprintln!(
                "Failed to get token information. Error: {:?}",
                GetLastError()
            );
            false
        }
    }
}

pub struct AdminChild {
    pub h_process: HANDLE,
}

impl AdminChild {
    pub fn wait(self) -> eyre::Result<u32> {
        unsafe {
            WaitForSingleObject(self.h_process, INFINITE);
            let mut code = 0u32;
            GetExitCodeProcess(self.h_process, &mut code)
                .map_err(|e| eyre!("Failed to get exit code: {}", e))?;
            CloseHandle(self.h_process)?;
            Ok(code)
        }
    }
}

/// Relaunches the current executable with administrative privileges, preserving arguments and console.
pub fn relaunch_as_admin() -> eyre::Result<AdminChild> {
    run_as_admin(&SameInvocationSameConsole)
}

/// Runs an invocable with administrative privileges using ShellExecuteExW.
pub fn run_as_admin(invocable: &impl Invocable) -> eyre::Result<AdminChild> {
    // Build a single space-separated string of arguments
    let params: OsString = invocable
        .args()
        .into_iter()
        .fold(OsString::new(), |mut acc, arg| {
            acc.push(arg);
            acc.push(" ");
            acc
        });

    // ---------------- ShellExecuteExW ----------------
    let verb = "runas".easy_pcwstr()?;
    let file = invocable.executable().easy_pcwstr()?;
    let params = params.easy_pcwstr()?;
    unsafe {
        let mut sei = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: params.as_ptr(),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        ShellExecuteExW(&mut sei).wrap_err("Failed to run as administrator")?;
        Ok(AdminChild {
            h_process: sei.hProcess,
        })
    }
}

/// Relaunches the current executable with administrative privileges using a specific CLI configuration.
pub fn relaunch_as_admin_with_cli(cli: &crate::cli::Cli) -> eyre::Result<AdminChild> {
    run_as_admin(cli)
}

/// Check if we're elevated, and relaunch if not
pub fn ensure_elevated() -> eyre::Result<()> {
    if is_elevated() {
        return Ok(());
    }
    warn!("Program needs to be run with elevated privileges.");
    info!("Relaunching as administrator...");
    match relaunch_as_admin() {
        Ok(child) => {
            info!("Spawned elevated process - waiting for it to finish…");
            let exit_code = child.wait()?;
            info!("Elevated process exited with code {exit_code}");
            std::process::exit(exit_code as i32);
        }
        Err(e) => {
            bail!("Failed to relaunch as administrator: {}", e);
        }
    }
}
