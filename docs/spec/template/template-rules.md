# Template Rules

This specification covers rules that apply to the `teamy-rust-cli` template repository itself rather than to repositories generated from it.

## Init Helper

template[init-script.present]
The template repository must contain an `init-other-repo.ps1` helper that copies the template into another repository.

template[init-script.omits-self]
The init helper must not copy `init-other-repo.ps1` into the destination repository.

template[init-script.omits-template-only-spec]
The init helper must omit template-only specification files from the destination repository.

template[init-script.omits-template-only-skills]
The init helper must omit template-only internal skills such as `.github/skills/initialize-from-teamy-rust-cli` from the destination repository.

template[init-script.removes-template-only-config]
The init helper must remove template-only spec wiring from the copied `.config/tracey/config.styx` file in the destination repository.

template[init-script.omits-build-artifacts]
The init helper must not copy template repository metadata or build artifacts such as `.git` and `target`.

template[init-script.preserves-license]
If the destination repository already contains a `LICENSE` file, the init helper must not clobber it.

template[init-script.preserves-readme]
If the destination repository already contains a `README.md` file, the init helper must not clobber it.

## Quality Gate

template[check-all.tests.exclude-tracy]
The template repository's `check-all.ps1` must run tests without enabling the `tracy` feature.

template[check-all.tests.avoid-firewall-prompt]
The template repository's quality gate must avoid enabling `tracy` during tests because Tracy can trigger a Windows firewall prompt that is inappropriate for routine automated validation.

## Placeholder Marking

template[todo.package-metadata]
Template-specific package metadata in `Cargo.toml` must have nearby `TODO` comments indicating that it should be replaced in generated repositories.

template[todo.crate-path]
Template-specific crate paths in Rust entrypoints and tests must have nearby `TODO` comments indicating that they should be renamed in generated repositories.

template[todo.repo-url]
Template-specific repository URLs in metadata or source code must have nearby `TODO` comments indicating that they should be replaced in generated repositories.

template[todo.path-env-vars]
Template-specific environment variable names and application directory names must have nearby `TODO` comments indicating that they should be replaced in generated repositories.

template[todo.spec-ids]
Template-specific tracey spec identifiers and source URLs must have nearby `TODO` comments indicating that they should be replaced or rewritten in generated repositories.

template[todo.readme]
Template README sections that are intended as scaffolding for generated repositories must have nearby `TODO` comments indicating that they should be rewritten.

template[todo.profile-helper-defaults]
Template profiling helper defaults that assume the template command surface must have nearby `TODO` comments indicating that they should be updated in generated repositories.