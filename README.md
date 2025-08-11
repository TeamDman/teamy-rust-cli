# Teamy Rust CLI

Opinionated starting point for building small/medium Rust command line applications.

## Features

- Structured CLI parsing (layered global + subcommand args)
- Windows UAC elevation helper that preserves stdout/stderr piping
- Console handle reuse to avoid detached window issues
- Logging via `tracing`
- Error reporting via `color-eyre`
- Organized module layout for scalability

## Elevation (Windows)

The helper in [`src/windows/win_elevation.rs`](./src/windows/win_elevation.rs):
- Detects if process is elevated
- Re-launches with admin rights when needed
- Reattaches to original console so output still streams to the invoking terminal

## Formatting

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