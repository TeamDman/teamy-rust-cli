# Teamy Rust CLI

Opinionated starting point for building small/medium Rust command line applications.

## Desired Outcomes

### CLI Parsing

The `clap` crate with `derive` feature is used as the primary description of the command line structure.  
The module structure has the `cli` module containing the `global_args.rs`, `mod.rs`, and `to_args.rs`, with the `cli::command` module containing additional modules to handle the actual commands.

The `to_args` implementation and `GlobalArgs` fields will come in use for other desired outcomes.

### Logging

The `tracing` and `tracing-subscriber` crates are used for logging. Passing `--debug` to the application will set the global args debug field to `true` which affects the tracing setup.

The tracing-subscriber configuration changes depending if it's a release or debug build.

### Elevation

By having the `ToArgs` trait implemented for our commands, we can easily relaunch the program when detected in an unelevated state.

The console handle is reused when relaunching to ensure that output goes to the terminal that the user expects, with the unelevated process exiting after the elevated one to ensure the console behaves as desired.

### Error handling

The `eyre` and `color-eyre` crates are used for handling error cases.

The `fn main` is responsible for early installation of the `color-eyre` behaviour.

All fallible functions return the fully-qualified `eyre::Result<T>` signature; we never use `use eyre::Result;` imports and we always qualify.

### Import statements

- One import per line
- Imports grouped into one big block
- See [`./rustfmt.toml`](./rustfmt.toml)

## Common crates

These are some crates I like to use.

- itertools
- nucleo
- ratatui
    - tachyonfx
    - throbber-widgets-tui
    - tui-textarea
- uom
    - https://github.com/TeamDman/understanding-uom
- rayon
- strum
- memmap2
- humansize
- humantime
- chrono
- cloud_terrastodon_user_input
- serde
    - serde_json