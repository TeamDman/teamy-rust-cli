# Introduction

This is the specification for the rust command line harness template.

This spec identifies the core components and behaviors that are shared across projects using this template.

## Binary & Resources

The binary contains embedded Windows resources for better integration with the OS.

> r[core.bin.resource.organization]
>
> Resources being bundled into the binary are located in the `resources` directory.

> r[core.bin.resource.manifest]
>
> There exists an `app.manifest` file defining application properties.

> r[core.bin.resource.rc]
>
> There exists an `app.rc` file that includes the manifest and defines the application icon.

> r[core.bin.icon]
>
> The CLI has at least one ICON resource defined.

> r[core.bin.git_rev]
>
> The binary embeds the short git revision of the workspace at compile time, accessible via the `GIT_REVISION` environment variable.

## Command Line Interface (CLI)

The CLI is designed for discoverability and consistency.

> r[core.cli.impl.dep.parser]
>
> Command line arguments are parsed using the `clap` crate with the `derive` feature.

> r[core.cli.version]
>
> The CLI automatically includes version information derived from the `Cargo.toml`.

> r[core.cli.async]
>
> The CLI execution is wrapped in a multi-threaded `tokio` runtime.

### Global Arguments

Arguments available to all subcommands.

> r[core.cli.global.debug]
>
> A global `--debug` flag is available to enable verbose logging and backtraces.

> r[core.cli.global.log_filter]
>
> A global `--log-filter` argument allows overriding the tracing subscriber directives.

> r[core.cli.global.log_file]
>
> A global `--log-file` argument allows specifying a file or directory for structured JSON logs. If a directory is provided, a timestamped filename is generated.

### Built-in Subcommands

The template provides standard management commands.

> r[core.cli.subcommand.home]
>
> The CLI provides a `home` subcommand to manage and locate the application home directory.

> r[core.cli.subcommand.cache]
>
> The CLI provides a `cache` subcommand to manage and locate the application cache directory.

## Paths & Storage

The application manages structured storage locations for configuration and transient data.

> r[core.paths.home.resolution]
>
> The application resolves a platform-appropriate home directory for configuration and data.

> r[core.paths.home.override]
>
> The home directory path can be overridden by a specific environment variable (e.g., `APP_HOME_DIR`).

> r[core.paths.cache.resolution]
>
> The application resolves a platform-appropriate cache directory for temporary or ephemeral data.

> r[core.paths.cache.override]
>
> The cache directory path can be overridden by a specific environment variable (e.g., `APP_CACHE_DIR`).

## Logging & Observability

Standardized logging facilitates debugging and monitoring.

> r[core.log.init]
>
> Structured logging (tracing) is initialized immediately after CLI parsing.

> r[core.log.ansi]
>
> On Windows, ANSI escape sequences are enabled for the console if supported.

> r[core.log.win.utf8_check]
>
> On Windows, the application warns the user if the system locale is not set to UTF-8.

> r[core.log.encoding.json]
>
> The logging system supports outputting logs in JSON format for machine readability.

## Quality & Verification

The harness includes tools and configurations to ensure code quality and correctness.

> r[core.quality.errors]
>
> The application uses `eyre` and `color-eyre` for rich, formatted error reporting and panic handling.

> r[core.quality.lints.disallowed]
>
> The project explicitly denies specifically disallowed methods and macros (e.g., `std::env::set_var`) via `clippy.toml`.

> r[core.quality.test.roundtrip]
>
> The CLI supports roundtrip verification, ensuring that generated `Cli` structures can be converted back into valid command-line arguments and re-parsed identically, verified using `arbitrary` based fuzzing.

> r[core.quality.check_script]
>
> A `check-all.ps1` script is provided to run formatting, linting, building, and testing in a single command.
