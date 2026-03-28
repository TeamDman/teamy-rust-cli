#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub mod cli;
pub mod logging_init;
pub mod paths;

use crate::cli::Cli;

/// Version string combining package version and git revision.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rev ",
    env!("GIT_REVISION"),
    ")"
);

/// Entrypoint for the program.
///
/// # Errors
///
/// This function will return an error if `color_eyre` installation, CLI parsing, logging initialization, or command execution fails.
///
/// # Panics
///
/// Panics if the CLI schema is invalid (should never happen with correct code).
pub fn main() -> eyre::Result<()> {
    #[cfg(windows)]
    {
        // Enable ANSI support on Windows
        // This fails in a pipe scenario, so we ignore the error
        let _ = teamy_windows::console::enable_ansi_support();

        // Warn if UTF-8 is not enabled on Windows
        #[cfg(windows)]
        teamy_windows::string::warn_if_utf8_not_enabled();
    };

    // Install color_eyre for better error reports
    color_eyre::install()?;

    // Parse command line arguments using figue
    // unwrap() is figue's intended CLI entry behavior:
    // it exits with proper codes for --help/--version/completions/parse-errors.
    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |cli| cli.args_os(std::env::args_os().skip(1)).strict())
            .help(move |help| {
                // TODO(template): replace the implementation git URL with the generated repository path.
                help.version(VERSION)
                    .include_implementation_source_file(true)
                    .include_implementation_git_url("TeamDman/teamy-rust-cli", env!("GIT_REVISION"))
            })
            .build(),
    )
    .run()
    .unwrap();

    // Initialize logging
    logging_init::init_logging(&cli.global_args)?;

    // Invoke whatever command was requested
    cli.invoke()?;
    Ok(())
}
