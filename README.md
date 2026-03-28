# Teamy Rust CLI

<!-- TODO(template): replace this README with project-specific documentation after copying the template. -->

[![crates.io](https://img.shields.io/crates/v/teamy-rust-cli.svg)](https://crates.io/crates/teamy-rust-cli)
[![license](https://img.shields.io/crates/l/teamy-rust-cli.svg)](https://crates.io/crates/teamy-rust-cli)

![Teamy Rust CLI media demo](resources/main.png)

Opinionated starting point for building small and medium Rust command line applications.

## What This Template Gives You

This template exists to remove the repeated setup work for a new CLI project. It ships with:

- `figue` + `facet` based argument parsing
- `--help` and `--version` support, including git revision in version output
- structured logging to stderr with optional NDJSON log files
- Windows app resources wired through `build.rs`
- CLI roundtrip fuzz tests
- tracey specification scaffolding under `.config/tracey` and `docs/spec`
- PowerShell helpers for initializing a new repository and running the full quality gate

## Quick Start

Copy the template into another repository:

```powershell
./init-other-repo.ps1 ../my-new-cli
```

Then update the obvious placeholders in the destination repository:

- package metadata in `Cargo.toml`
- environment variable names in `src/paths/mod.rs`
- repository URL in `src/lib.rs`
- README text and examples

## Example Usage

Inspect the generated CLI surface:

```powershell
cargo run -- --help
```

Show the resolved application home directory:

```powershell
cargo run -- home show
```

Show the resolved cache directory:

```powershell
cargo run -- cache show
```

Write structured logs to disk while still logging to stderr:

```powershell
cargo run -- --log-file .\logs home show
```

## Environment Variables

<!-- TODO(template): replace the environment variable names below with project-specific names. -->

The template currently recognizes these environment variables:

- `APP_HOME_DIR`: overrides the resolved application home directory
- `APP_CACHE_DIR`: overrides the resolved cache directory
- `RUST_LOG`: provides a tracing filter when `--log-filter` is not supplied

These names are placeholders and should be renamed in the generated project.

## Quality Gate

Run the standard validation flow with:

```powershell
./check-all.ps1
```

That script runs formatting, clippy, build, tests, and local tracey validation.

For Tracy profiling, run:

```powershell
./run-tracing.ps1 home show
```

<!-- TODO(template): update the example profiling command above to match the generated project's command surface. -->

## Repository Layout

```text
. # Some files omitted
├── .config/tracey/config.styx # Local tracey specification wiring
├── build.rs # Adds exe resources and embeds git revision
├── Cargo.toml # Package metadata, dependencies, lint policy
├── check-all.ps1 # Formatting, linting, build, tests, tracey validation
├── docs/spec # Human-readable requirements for the repository and CLI
├── resources # Windows resources used by build.rs
├── src # Rust source code
├── tests # CLI roundtrip fuzz tests
└── update.ps1 # Convenience install helper
```
