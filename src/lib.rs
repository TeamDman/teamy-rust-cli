mod command_path;
mod entrypoint;
mod failure_record;
mod safe_failure;
pub use entrypoint::entrypoint;
pub mod cli;
pub mod elapsed_duration;
pub mod logging_init;
pub mod paths;
#[cfg(windows)]
mod windows_startup;
