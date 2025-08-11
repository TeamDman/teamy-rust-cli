use crate::cli::global_args::GlobalArgs;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Console::*;
use windows::core::w;

/// Reuses the console of the parent process if requested via command line args.
/// This must be called before any logging initialization or stdout/stderr usage.
pub fn reuse_console_if_requested(global_args: &GlobalArgs) {
    let Some(pid) = global_args.console_pid else {
        if global_args.debug {
            eprintln!("No console PID provided, skipping console reuse.");
        }
        return;
    };
    if global_args.debug {
        eprintln!("Reusing console with PID: {pid}");
    }

    unsafe {
        // Detach from (non-existent) default console just in case
        let _ = FreeConsole();

        // Try to attach to the parent console
        if let Err(e) = AttachConsole(pid) {
            // If attaching fails, allocate a new console as fallback
            let _ = AllocConsole();
            eprintln!(
                "Failed to attach to console with PID {pid}, allocated a new console instead. Error: {e:?}"
            );
            return;
        }

        // Re-open standard handles so Rust's std::io uses the console.
        let con_out = CreateFileW(
            w!("CONOUT$"),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        );
        let con_in = CreateFileW(
            w!("CONIN$"),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        );

        if let Ok(con_out) = con_out {
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, con_out);
            let _ = SetStdHandle(STD_ERROR_HANDLE, con_out);

            // Optional: enable ANSI again
            let mut mode = CONSOLE_MODE::default();
            if GetConsoleMode(con_out, &mut mode).is_ok() {
                let _ = SetConsoleMode(
                    con_out,
                    mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT,
                );
            }
        }

        if let Ok(con_in) = con_in {
            let _ = SetStdHandle(STD_INPUT_HANDLE, con_in);
        }
    }
}
