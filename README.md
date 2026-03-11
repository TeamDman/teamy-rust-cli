# Teamy Rust CLI

Opinionated starting point for building small/medium Rust command line applications.

## Getting Started

Everything but this README.md file and the LICENSE file can be copied to a new project to get off the ground easily.

```shell
teamy-rust-cli on  main [!] is 📦 v0.2.0 via 🦀 v1.94.0-nightly 
❯ ls --tree --git-ignore
. # Some files omitted
├── build.rs # Adds exe resources (icon, manifest) and git revision
├── Cargo.lock # Pins exact version info for crates, no special behaviour
├── Cargo.toml # Lint rules, includes dependency for build.rs
├── check-all.ps1 # Formatting, pedantic linting, build checking
├── clippy.toml # Forbidding undesired method and macro usage
├── LICENSE # The license for this template, do not clobber destination if exists when applying template
├── README.md # This file :D
├── resources # build.rs uses when creating .exe
│  ├── app.manifest # Can be necessary when using specific windows controls
│  ├── app.rc # Declare resources to be bundled in .exe like icons
│  ├── main.ico # Icon file
│  ├── main.png # Original icon image
│  ├── make-all-icons.ps1 # Builds all declared images into .ico files
│  └── make-icon.ps1 # Helper fn script for above
├── rustfmt.toml # Opinionated formatting rules
├── src # Rust source code
│  ├── cli # CLI holder
│  │  ├── cache
│  │  │  ├── clean # my-app cache clean
│  │  │  └── show # my-app cache show
│  │  ├── global_args.rs # Arguments that are applicable to all commands
│  │  └── home
│  │     └── show # my-app home show
│  ├── lib.rs # Entrypoint
│  ├── logging
│  │  ├── logging_config.rs # Configure object holds tracing filter and optional json output file
│  │  └── logging_init.rs # Initialize tracing
│  ├── main.rs # CLI entrypoint calls lib.rs#main
│  └── paths
│     ├── app_home.rs # Helper gives dir to put roaming data; configs, preferences
│     └── cache.rs # Helper gives dir to put local data; cached operation outputs, downloads
├── tests
│  └── cli_fuzzing.rs # Round-trip testing for Cli arbitrary implementation
└── update.ps1 # Convenience: cargo install --path .
```
