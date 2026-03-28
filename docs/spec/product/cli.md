# CLI

<!-- TODO(template): replace this spec with the generated project's real command surface and environment variable names. -->

This specification covers the current user-facing command line behavior exposed by `teamy-rust-cli`.

## Command Surface

cli[command.surface.core]
The CLI must expose the `cache` and `home` command groups.

cli[command.surface.cache]
The `cache` command group must expose the `show`, `open`, and `clean` subcommands.

cli[command.surface.home]
The `home` command group must expose the `show` and `open` subcommands.

## Parser Model

cli[parser.args-consistent]
The structured CLI model must serialize to command line arguments consistently for parse-safe values.

cli[parser.roundtrip]
The structured CLI model must roundtrip through argument serialization and parsing for parse-safe values.

## Path Resolution

cli[path.app-home.env-overrides-platform]
If `APP_HOME_DIR` is set to a non-empty value, it must take precedence over the platform-derived application home directory.

cli[path.cache.env-overrides-platform]
If `APP_CACHE_DIR` is set to a non-empty value, it must take precedence over the platform-derived cache directory.