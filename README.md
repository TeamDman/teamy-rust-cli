# Teamy Rust CLI

<!-- TODO(template): replace this README with project-specific documentation after copying the template. -->

[![crates.io](https://img.shields.io/crates/v/teamy-rust-cli.svg)](https://crates.io/crates/teamy-rust-cli)
[![license](https://img.shields.io/crates/l/teamy-rust-cli.svg)](https://crates.io/crates/teamy-rust-cli)

<image height="100" src="./resources/main.png"/>

Opinionated starting point for building small and medium Rust command line applications.

## What This Template Gives You

This template exists to remove the repeated setup work for a new CLI project. It ships with:

- `figue` + `facet` based argument parsing
- top-level output rendering with `text`, `json`, and `csv` formats
- `--help` and `--version` support, including git repository URL, branch, revision, worktree status, and local-time build timestamp in version output
- structured logging to stderr with optional NDJSON log files
- color-eyre error chains, source locations and panic reports
- Windows app resources wired through `build.rs`
- CLI roundtrip fuzz tests
- `init` scaffolding command for creating new Teamy Rust CLI repositories
- PowerShell helpers for running the full quality gate and temporarily supporting the legacy initializer
- cooperative cancellation helpers from `teamy-cancellation`

### Cancellation integration

The template uses crates.io `teamy-cancellation` 0.7.1 with its `async` and `figue` features.
Command dispatch cancels pending futures on Ctrl+C or a stop-after condition.
Async subcommands must be safe to abandon at each await point: cancellation
cannot interrupt synchronous work within a poll. Required completion work needs
an explicit operation-local deferral before introducing cancellable awaits; the
independent `deferral` feature is not enabled by this template.

Global arguments use `teamy_cancellation::StopAfterArgs` directly. The cancellation,
Facet and Figue versions are pinned together in `Cargo.toml`.
## Quick Start

Scaffold the template into another repository:

```powershell
cargo run -- init ../my-new-cli
```

Then update the obvious placeholders in the destination repository:

- package metadata in `Cargo.toml`
- environment variable names in `src/paths/mod.rs`
- repository URL in `Cargo.toml` (also used for help source links)
- README text and examples

The initializer creates the destination directory if needed, excludes `.git`,
`target`, the template initialization skill, and the legacy `init-other-repo.ps1`
script, and preserves an existing destination `README.md` or `LICENSE`.
Existing generated files are not overwritten unless `--force` is passed:

```powershell
cargo run -- init ../my-existing-cli --force
```

For transition, `init-other-repo.ps1` remains in this repository as a
compatibility entrypoint, but new documentation and automation should use
`teamy-rust-cli init`.

## Example Usage

Inspect the generated CLI surface:

```powershell
cargo run -- --help
```

Show the resolved application home directory:

```powershell
cargo run -- home show
```

Render the same command as JSON:

```powershell
cargo run -- --output-format json home show
```

Show the resolved cache directory:

```powershell
cargo run -- cache show
```

Write structured logs to disk while still logging to stderr:

```powershell
cargo run -- --log-file .\logs\capture.ndjson home show
```

## Command Output

Commands that return structured data render through a shared top-level output layer.

- Use `--output-format text`, `--output-format json`, or `--output-format csv` to choose the format explicitly.
- If `--output-format` is omitted, the CLI defaults to `text` in an interactive terminal and `json` when stdout is redirected.
- Commands that only perform side effects, such as `open`, return no structured stdout payload. The `cache clean` command is a placeholder for project-specific cache policy.

## Environment Variables

<!-- TODO(template): replace the environment variable names below with project-specific names. -->

The template currently recognizes these environment variables:

- `APP_HOME_DIR`: overrides the resolved application home directory
- `APP_CACHE_DIR`: overrides the platform-derived cache directory
- `RUST_LOG`: provides a tracing filter when `--log-filter` is not supplied

These names are placeholders and should be renamed in the generated project.

## Runtime expectations and performance evidence

Each subcommand returns `CliOutput` with an explicit
`elapsed_duration_warn_threshold: Option<Duration>`. Constructors require the
budget; parent dispatch forwards it. The top level captures an `Instant` before
invocation, renders output, then emits a structured warning only when elapsed
exceeds the returned budget. This completion-time check does not change results
or act as a live watchdog. Invocation errors have no returned budget.
Path inspection receives one second; detached folder opening receives three.
Scaffolding receives three seconds plus 20 ms per inventory file. `None` explicitly
exempts unbounded serve/watch and user-interactive work; document each exemption.
See the [contract and downstream migration checklist](docs/command-completion-budgets.md).

The NDJSON layer records span-close busy/idle durations; stderr keeps its existing
event formatting. Build first, then collect an explicitly named log file with an
info/debug filter that enables the relevant spans. The file path is created or
appended. Read the [performance methodology](https://teamdman.github.io/teamy-docs/performance-analysis.html)
for timing boundaries, phase instrumentation, Tracy captures, teamy-profiler and
correctness-preserving comparisons.

## Quality Gate

Run the standard validation flow with:

```powershell
./check-all.ps1
```

That script runs formatting, linting, build, and tests

For Tracy profiling, run:

```powershell
./run-profiler.ps1 home show
```

<!-- TODO(template): update the example profiling command above to match the generated project's command surface. -->

## Profiling contract

Profiling is opt-in through the `tracy` Cargo feature. It enables the explicitly
configured `tracing-tracy` transport. The `tracing-subscriber` and `tracing-tracy`
dependencies disable their defaults and list the capabilities used here.
Human stderr logs, optional NDJSON logs, and Tracy use independent filters.
The Tracy layer uses a fixed trace-level filter so a user-facing `--log-filter`
does not silently remove profiling spans; profiling-only frame markers are
excluded from the human and structured log layers.

Run a release capture through the shared harness:

```powershell
./run-profiler.ps1 home show
```

The wrapper delegates to `teamy-profiler run cargo`, forwards command
arguments, and writes the capture, logs, manifest, and top-span CSV under the
repository's `tracy/` directory. Install `teamy-profiler` and make sure
`tracy-capture.exe` is available on `PATH`. Use the profiler's `--dry-run`
option to audit feature forwarding and artifact paths without launching a
capture. Active repositories should periodically compare their Cargo feature
graph, logging layers, wrapper, and README to this template and the profiler
README.

## Repository Layout

```text
. # Some files omitted
├── build.rs # Adds exe resources and embeds git repository/build metadata
├── Cargo.toml # Package metadata, dependencies, lint policy
├── check-all.ps1 # Formatting, linting, build, tests
├── init-other-repo.ps1 # Legacy initializer kept temporarily during CLI migration
├── resources # Windows resources used by build.rs
├── src # Rust source code
├── tests # CLI roundtrip fuzz tests
└── update.ps1 # Convenience install helper
```

## Logging and progress

See [the logging and progress contract](docs/logging-progress.md) for independent filters/formats, precedence, safe error output and operation-specific structured events.

## Deterministic initialization

`init PATH --name new-tool --dry-run` previews the complete file plan without writes. Omit PATH with `--name` to create NAME in the current directory. Identity replacement updates package/crate names, home/cache environment names and source links. Supply `--repository-url https://github.com/OWNER/REPO` only for metadata; no remote repository is created. MPL-2.0 is the fresh-project default and existing README/LICENSE are preserved. Overlapping source/destination paths and destination symlinks are rejected. Template inputs are curated; local caches, credentials and logs are excluded.

## Windows DPI awareness

The embedded application manifest enables Common Controls v6 and selects
`PerMonitorV2,PerMonitor` with legacy `true/pm` fallback. Windows applies awareness
before any windows are created; native dialogs avoid DPI-unaware bitmap stretching.
The build embeds these resources into binaries, examples, and test executables.
Generated projects inherit this manifest and its regression checks. Existing CLIs
should adopt it individually after checking their custom UI scaling behavior.
No execution-level declaration is added or changed.

`cargo test --test windows_manifest` verifies the actual built Windows CLI and test executable's
RT_MANIFEST resource. For an installed or release binary:
`./resources/check-windows-manifest.ps1 -Executable path/to/tool.exe`.
