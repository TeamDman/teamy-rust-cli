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

Command line structure is fuzz-tested by [tests/cli_fuzzing.rs](./tests/cli_fuzzing.rs).

## Expected structure

```
src/cli/command/
  model/ # a demonstrative subcommand, this is a placeholder noun for whatever subcommands other projects may have
    mod.rs              # ModelArgs (clap::Args) + ModelCommand enum
    model_command.rs    # ModelCommand (clap::Subcommand)
    model_list_command.rs
    model_train_command.rs
    kind/ # a demonstrative nested subcommand, this is a placeholder noun for whatever subcommands other projects may have
      mod.rs            # ModelKindArgs (clap::Args) + ModelKindCommand enum
      model_kind_command.rs
      model_kind_list_command.rs
tests/
  cli_fuzzing.rs
CONTRIBUTING.md # a copy of this file
rustfmt.toml
README.md
Cargo.toml
Cargo.lock
.gitignore
```

## Refactoring Guide

When restructuring existing CLI commands into the standard pattern, follow these steps:

### 1. Plan the File Structure

Before making changes, document the expected file tree. For each command:
- Commands with subcommands need: `{name}_args.rs`, `{name}_command.rs`, and per-variant `{name}_{variant}_args.rs` files
- Simple commands (no subcommands) need only: `{name}_args.rs`
- All commands need a `mod.rs` that re-exports the primary types

Example planning document:
```markdown
After refactor:
src/cli/command/
  mod.rs
  command.rs          # Top-level Command enum
  host/
    mod.rs            # Re-exports HostCommand
    host_args.rs      # Wrapper struct with #[command(subcommand)]
    host_command.rs   # HostCommand enum
    host_add_args.rs
    host_remove_args.rs
    host_list_args.rs
```

### 2. Create the Module Structure

For each command with subcommands:

**{name}/mod.rs** - Re-export pattern:
```rust
pub mod {name}_args;
pub mod {name}_command;
pub mod {name}_{variant1}_args;
pub mod {name}_{variant2}_args;

pub use {name}_command::{Name}Command;
```

**{name}/{name}_command.rs** - Command enum:
```rust
#[derive(Debug, Subcommand)]
pub enum {Name}Command {
    Variant1({Name}Variant1Args),
    Variant2({Name}Variant2Args),
}

impl {Name}Command {
    pub fn invoke(self, /* params */) -> eyre::Result<()> {
        match self {
            Self::Variant1(args) => args.invoke(/* params */),
            Self::Variant2(args) => args.invoke(/* params */),
        }
    }
}
```

**{name}/{name}_{variant}_args.rs** - Leaf command:
```rust
#[derive(Debug, Args)]
pub struct {Name}{Variant}Args {
    // fields
}

impl {Name}{Variant}Args {
    pub fn invoke(self, /* params */) -> eyre::Result<()> {
        // implementation
    }
}
```

### 3. Wire Up the Top-Level Command

In `command.rs`, import and use the command enum (not the args wrapper):
```rust
use crate::cli::command::host::host_command::HostCommand;

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Host(HostCommand),
}
```

### 4. Migration Checklist

- [ ] Document expected file structure before coding
- [ ] Create all `*_command.rs` files for enums with subcommands
- [ ] Create all `*_args.rs` files for leaf command implementations
- [ ] Move `invoke()` logic from command enums to individual args files
- [ ] Update `mod.rs` to re-export command enums (not args wrappers)
- [ ] Update parent `command.rs` to use command enums directly
- [ ] Run `cargo build` to verify compilation
- [ ] Test each command variant to ensure behavior is preserved
- [ ] Run `cargo fmt`

