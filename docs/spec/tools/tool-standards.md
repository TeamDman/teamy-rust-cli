# Tool Standards

<!-- TODO(template): replace the project name references in this spec after copying the template. -->

This document translates the applicable command-line and logging standards into local tracey requirements for `teamy-rust-cli`.

## Command Line Interface

tool[cli.version.includes-semver]
The CLI must report the semantic version from the project manifest.

tool[cli.version.includes-git-revision]
The CLI must report the current git revision alongside the semantic version.

tool[cli.help.describes-behavior]
The CLI help output must describe the expected behavior of the program and its commands.

tool[cli.help.describes-argv]
The CLI help output must describe the command line arguments accepted by the program.

tool[cli.help.describes-environment]
The CLI help output must describe environment variables that affect program behavior.

tool[cli.help.position-independent]
The CLI must support requesting help from nested command positions.

## Logging

tool[logging.stderr-output]
The program must send logs to stderr.

tool[logging.file-path-option]
The program must support optionally writing logs to a user-provided path on disk.

tool[logging.file-structured-ndjson]
When the program writes logs to disk, the file output must use a structured NDJSON representation.

## Quality Gate

tool[tests.exclude-tracy-feature]
The repository quality gate must run tests without enabling the `tracy` feature.

tool[tests.avoid-tracy-firewall-prompt]
The repository quality gate must avoid enabling `tracy` during tests because Tracy can trigger a Windows firewall prompt that is inappropriate for routine automated validation.