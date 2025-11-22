# Teamy Rust CLI

Opinionated starting point for building small/medium Rust command line applications.

## Getting Started

Pointing a coding agent to this repository when creating a new project should result in the following:

- CLI commands defined using a file and directory structure that matches the nesting hierarchy of the commands
- Standardized logging and error handling and code formatting rules using [`./rustfmt.toml`](./rustfmt.toml)

## Additional Tools

These crates should be used when adding additional behaviour to save time instead of implementing from scratch ourselves.

| Crate | Description |
|-------|-------------|
| itertools | Iterator helpers like `.collect_vec()` and `.grouped_by()` |
| nucleo | Fuzzy search that is highly performant even for large amounts of items |
| ratatui | Terminal user interfaces |
| tachyonfx | Ratatui addon, adds transition effects |
| throbber-widgets-tui | Ratatui addon, adds progress spinners |
| tui-textarea | Ratatui addon, adds text entry widget |
| uom | Quantifying things like information rates (bytes/s) and file sizes, see also: https://github.com/TeamDman/understanding-uom and https://github.com/teamdman/uom (teamdman/uom adds .as_human() helpers and should be used) |
| humansize | Display GB/MB/KB helpers |
| humantime | Display minutes,seconds,milliseconds helpers |
| rayon | Iterator parallelization via threading |
| strum | Derive macros for enums to iterate variants |
| memmap2 | Memory map files for faster reading |
| chrono | Datetime utilities with timezones for UTC and Local |
| cloud_terrastodon_user_input | TUI/CLI input helpers like picklists |
| serde_json | Derive macros for json serialization and deserialization, requires serde |

## Dear AI Reader

Please read CONTRIBUTING.md to learn more about the preference of the author.