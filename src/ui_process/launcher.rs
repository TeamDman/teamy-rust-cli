use std::ffi::OsString;
use std::io::Read;
use std::io::Write;
use std::io::{self};
use std::path::Path;
use std::process::ExitStatus;

/// Only the explicit UI mode creates a console; fixtures run without one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChildConsole {
    Hidden,
    NewConsole,
}

pub(super) struct Spawned {
    pub process: Process,
    pub input: Box<dyn Write + Send>,
    pub output: Box<dyn Read + Send>,
    pub bootstrap: Option<Bootstrap>,
}

#[cfg(not(windows))]
#[derive(Debug)]
pub(super) struct Bootstrap;

#[cfg(not(windows))]
#[derive(Debug)]
pub(super) struct Process(std::process::Child);

#[cfg(not(windows))]
impl Process {
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.0.try_wait()
    }
    pub fn kill(&mut self) -> io::Result<()> {
        self.0.kill()
    }
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.0.wait()
    }
}

#[cfg(not(windows))]
pub(super) fn spawn(
    program: &Path,
    args: &[OsString],
    _console: ChildConsole,
) -> io::Result<Spawned> {
    use std::process::Command;
    use std::process::Stdio;
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other("missing child pipe"));
    };
    Ok(Spawned {
        process: Process(child),
        input: Box::new(input),
        output: Box::new(output),
        bootstrap: None,
    })
}

#[cfg(windows)]
mod windows_launcher;
#[cfg(windows)]
pub(super) use windows_launcher::Bootstrap;
#[cfg(windows)]
pub(super) use windows_launcher::Process;
#[cfg(windows)]
pub(super) use windows_launcher::spawn;
