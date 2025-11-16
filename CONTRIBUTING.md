# CONTRIBUTING

This document outlines the practices that must be adopted by entities working on this code base.

## Formatting

Run `cargo fmt` after adding or removing `use` statements.

## Error Handling

The `eyre` and `color-eyre` crates are used for handling error cases.

The `fn main` is responsible for early installation of the `color-eyre` behaviour.

All fallible functions return the fully-qualified `eyre::Result<T>` signature; we never use `use eyre::Result;` imports and we always qualify.

## Command Line Argument Structure

Commands are organized hierarchically with a consistent naming and file structure pattern:

- **Root**: `Cli` struct in `src/cli/mod.rs` derives `clap::Parser` and contains:
  - A `#[clap(flatten)]` field for `GlobalArgs` (shared flags like verbosity, JSON output, etc.)
  - A `#[clap(subcommand)]` field for the top-level `CliCommand` enum
- **Top-level**: `CliCommand` enum derives `clap::Subcommand` and is defined in `src/cli/command/mod.rs`
- **Subcommands**: Each subcommand (e.g., `model`) gets its own directory with a `mod.rs` containing:
  - `{Name}Args` struct deriving `clap::Args`
  - A `#[clap(subcommand)]` field containing a `{Name}Command` enum
- **Command enums**: The `{Name}Command` enum derives `clap::Subcommand` and defines sub-subcommands
- **Leaf commands**: Terminal commands (e.g., `list`, `train`) are defined in separate files like `{parent}_{action}_command.rs` with their own `{Action}Args` structs
- **Nested subcommands**: Sub-subcommands follow the same pattern, getting their own subdirectory (e.g., `model/kind/`)

Each layer implements `invoke()` for execution logic and `ToArgs` for serialization. All argument structs derive `clap::Args` or `clap::Subcommand`, `arbitrary::Arbitrary`, `PartialEq`, and `Debug`.

**Example structure**:
```
src/cli/command/
  model/
    mod.rs              # ModelArgs (clap::Args) + ModelCommand enum
    model_command.rs    # ModelCommand (clap::Subcommand)
    model_list_command.rs
    model_train_command.rs
    kind/
      mod.rs            # ModelKindArgs (clap::Args) + ModelKindCommand enum
      model_kind_command.rs
      model_kind_list_command.rs
```
