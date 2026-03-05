#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub mod cli;
pub mod logging;
pub mod paths;

use crate::cli::Cli;

fn help_description(args: &[String]) -> String {
    let has_subcommand = args.iter().any(|arg| !arg.starts_with('-'));
    let program_name = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "teamy-rust-cli.exe".to_owned());

    let context_path = args
        .iter()
        .filter(|arg| !arg.starts_with('-'))
        .cloned()
        .collect::<Vec<_>>();

    let current_impl = crate::cli::docs::help_implementation_source(&context_path);

    let hints = if has_subcommand {
        let mut level_hints = crate::cli::docs::help_invocation_hints_at_level(
            &program_name,
            &context_path,
            Some(&context_path),
        );
        if level_hints.is_empty() && !context_path.is_empty() {
            level_hints = crate::cli::docs::help_invocation_hints_at_level(
                &program_name,
                &context_path[..context_path.len() - 1],
                Some(&context_path),
            );
        }
        level_hints
    } else {
        crate::cli::docs::help_invocation_hints(&program_name)
    };

    if hints.is_empty() && current_impl.is_none() {
        return String::new();
    }

    let mut description = String::new();
    if let Some(path) = current_impl {
        description.push_str("Implementation:\n");
        description.push_str("  ");
        description.push_str(path);
        description.push('\n');
    }

    if !hints.is_empty() {
        if !description.is_empty() {
            description.push('\n');
        }
        description.push_str("More help:\n");
        for hint in hints {
            description.push_str("  ");
            description.push_str(&hint);
            description.push('\n');
        }
    }

    description.trim_end().to_owned()
}

fn normalized_cli_args() -> Vec<String> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let has_help_flag = args.iter().any(|arg| arg == "--help" || arg == "-h");
    if !has_help_flag && matches!(args.last().map(String::as_str), Some("help")) {
        if args.len() == 1 {
            args[0] = "--help".to_owned();
        } else {
            let _ = args.pop();
            args.push("--help".to_owned());
        }
    }
    args
}

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
    // Install color_eyre for better error reports
    color_eyre::install()?;

    let normalized_args = normalized_cli_args();
    let extra_help_description = help_description(&normalized_args);

    // Parse command line arguments using figue
    // unwrap() is figue's intended CLI entry behavior:
    // it exits with proper codes for --help/--version/completions/parse-errors.
    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |c| c.args(normalized_args))
            .help(move |h| {
                let base = h.version(VERSION);
                if extra_help_description.is_empty() {
                    base
                } else {
                    base.description(extra_help_description)
                }
            })
            .build(),
    )
    .run()
    .unwrap();

    // Initialize logging
    logging::init_logging(&cli.global.logging_config()?)?;

    #[cfg(windows)]
    {
        // Enable ANSI support on Windows
        // This fails in a pipe scenario, so we ignore the error
        let _ = teamy_windows::console::enable_ansi_support();

        // Warn if UTF-8 is not enabled on Windows
        #[cfg(windows)]
        teamy_windows::string::warn_if_utf8_not_enabled();
    };

    // Invoke whatever command was requested
    cli.invoke()?;
    Ok(())
}
