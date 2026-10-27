# Teamy Rust CLI


<!-- TODO(template): replace this README with project-specific documentation after copying the template. -->

[![crates.io](https://img.shields.io/crates/v/teamy-rust-cli.svg)](https://crates.io/crates/teamy-rust-cli)
[![license](https://img.shields.io/crates/l/teamy-rust-cli.svg)](https://crates.io/crates/teamy-rust-cli)


<image height="100" src="./resources/main.png"/>

Opinionated starting point for building small and medium Rust command line applications.

## What This Template Gives You

This template exists to remove the repeated setup work for a new CLI project. It ships with:

- `figue` + `facet` based argument parsing using Teamy's published forks
- top-level output rendering with `text`, `json`, and `csv` formats
- `--help` and `--version` support, including git repository URL, branch, revision, worktree status, and local-time build timestamp in version output
- structured logging to stderr with optional NDJSON log files
- Windows app resources wired through `build.rs`
- CLI roundtrip fuzz tests
- `init` scaffolding command for creating new Teamy Rust CLI repositories
- PowerShell helpers for running the full quality gate and temporarily supporting the legacy initializer
- cancellation signals and linked execution scopes from `teamy-cancellation`

## Quick Start

Scaffold the template into another repository:

```powershell
cargo run -- init ../my-new-cli
```

Then update the obvious placeholders in the destination repository:

- package metadata in `Cargo.toml`
- environment variable names in `src/paths/mod.rs`
- repository URL in `src/lib.rs`
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
cargo run -- --log-file .\logs home show
```

## Command Output

Commands that return structured data render through a shared top-level output layer.

- Use `--output-format text`, `--output-format json`, or `--output-format csv` to choose the format explicitly.
- If `--output-format` is omitted, the CLI defaults to `text` in an interactive terminal and `json` when stdout is redirected.
- Commands that only perform side effects, such as `open` and `clean`, return no structured stdout payload.

JSON and text output stream to the writer under the invocation's cancellation
scope. JSON also checks cancellation during traversal and scalar encoding.
CSV currently buffers its rendered document, then uses bounded cancellation-aware
writes. All formats check cancellation while writing and before flushing, keeping
the original cancellation reason and respecting protected scopes. Cancellation
or an output failure can leave a partial document; synchronous checks cannot
interrupt a blocked underlying write or flush.

## Environment Variables

<!-- TODO(template): replace the environment variable names below with project-specific names. -->

The template currently recognizes these environment variables:

- `APP_HOME_DIR`: overrides the resolved application home directory
- `APP_CACHE_DIR`: overrides the platform-derived cache directory
- `RUST_LOG`: provides a tracing filter when `--log-filter` is not supplied

These names are placeholders and should be renamed in the generated project.

## Dependencies and Cancellation

The canonical Rust names alias TeamDman's independent published forks:

- `facet` and `facet-pretty` use their `teamy-facet-* =0.50.0-rc.7` packages.
- `facet-csv` uses `teamy-facet-csv =0.50.0-rc.8`.
- `facet-json` uses `teamy-facet-json =0.50.0-rc.8`; the shared `teamy-facet-format` dependency also uses `=0.50.0-rc.8`.
- `figue` uses `teamy-figue =6.0.0-rc.4`, with `teamy-figue-attrs =6.0.0-rc.3`.
- `teamy-cancellation =0.6.0` supplies matching Figue arguments, cancellation execution scopes, and synchronous I/O checkpoints.

These are forks, not the upstream Facet or Figue releases. The mixed Facet
versions are deliberate: the JSON/format release adds cooperative cancellation
and direct writer output while retaining the matching Facet rc.7 core types.

The process owns a shared `CancellationToken` for Ctrl-C and stop-after requests.
Invocation receives a bounded `CancellationScope`; its linked cancellable future
checks cancellation at poll boundaries. Pass that supplied scope into new async
operations, and use its nested scope helpers rather than starting an independent
root. Synchronous work still needs explicit checkpoints or yields. Enable
`teamy-cancellation`'s `deferral` feature for operations that must finish a
submitted mutation and related cleanup in a bounded protected scope. Asynchronous
shutdown owners should use a cooperative scope and retain their required child
work.

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
