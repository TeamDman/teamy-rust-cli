---
name: initialize-from-teamy-rust-cli
description: Use when scaffolding a new Rust CLI from the teamy-rust-cli template. Covers copying the template into a destination repo, preserving destination README or LICENSE when present, replacing template placeholders, and validating that the generated project is no longer template-branded.
argument-hint: Describe the destination path, intended crate name, binary name, repository URL, and any app-specific home/cache env var names.
---

# Initialize From teamy-rust-cli

## When to Use
- You want to start a new Rust CLI from the `teamy-rust-cli` template repository.
- You need the agent to do both the copy step and the mandatory post-copy cleanup.
- You want a concrete checklist for removing template branding and placeholder defaults.
- You are working on Windows and can run the template's PowerShell helper.

## Goal
Create a usable new CLI repository from `teamy-rust-cli` without carrying forward template-only files, placeholder names, or outdated examples.

The result should build, show the correct help text, use the new crate identity everywhere, and pass the normal validation flow.

## Procedure
1. Confirm the destination and project identity.
   Collect the destination path, crate name, repository URL, human-readable description, and the app-specific home and cache environment variable names.
2. Copy the template with the helper script.
   From the `teamy-rust-cli` repository, run `./init-other-repo.ps1 <destination>`. This copies the template while excluding `.git`, `target`, and the template-only skill directory for this workflow.
3. Verify what the helper intentionally preserves.
   If the destination already had `README.md` or `LICENSE`, the helper leaves those files in place. Do not assume the copied template README or license exists in that case.
4. Replace package metadata.
   Update `Cargo.toml` fields that still describe the template, especially `name`, `description`, `repository`, `documentation`, and `keywords`.
5. Replace crate-path references.
   Rename the template crate path in `src/main.rs` and `tests/cli_fuzzing.rs` so they reference the new crate.
6. Replace repository identity and source links.
   Update `src/lib.rs` so the implementation git URL points at the generated repository instead of `TeamDman/teamy-rust-cli`.
7. Replace path and environment-variable placeholders.
   Update `src/paths/mod.rs` so the app home and cache environment variable names and directory names match the new project.
8. Rewrite the CLI surface scaffolding.
   Replace the demonstration description and example command groups in `src/cli/mod.rs` with the real command surface for the new tool.
9. Rewrite the README and examples.
    Replace the template README text, example commands, and environment variable names with project-specific documentation.
10. Update the profiling helper defaults.
    Change the default traced command in `run-profiler.ps1` so it traces a real command for the new CLI instead of `home show`.
11. Search for leftover template markers.
    Run a repo-wide search for `TODO(template)`, `teamy-rust-cli`, `TeamDman/teamy-rust-cli`, `APP_HOME_DIR`, and `APP_CACHE_DIR`. Either replace each remaining hit or confirm it is intentionally kept.
12. Validate the generated project.
    Run `cargo run -- --help` to check the CLI surface, then run `./check-all.ps1` to validate formatting, clippy, build, and tests.

## Recommended Commands

```powershell
./init-other-repo.ps1 ../my-new-cli
```

```powershell
rg 'TODO\(template\)|teamy-rust-cli|TeamDman/teamy-rust-cli|APP_HOME_DIR|APP_CACHE_DIR' .
```

```powershell
cargo run -- --help
./check-all.ps1
```

## Decision Points

### Existing Repository Or Fresh Directory
- If the destination is an existing repository, assume `README.md` and `LICENSE` may already be authoritative and should not be clobbered.
- If the destination is a fresh directory, expect to rewrite the copied README immediately and decide whether to keep the template license.

### Rename Everything Now Or Stage The Cleanup
- Prefer renaming all template placeholders in the same session as the copy. This avoids building more code on top of placeholder crate names and env vars.
- If you must stage the cleanup, finish crate identity, repository URL, env vars, and CLI description before any feature work.

### Keep Or Replace Template Features
- Keep the shared infrastructure by default: logging, fuzz tests, Windows resource wiring, and quality-gate scripts.
 - Replace only the product-facing surface area: metadata, docs, command groups, env var names, and default traced command.

## Checklist By File
- `Cargo.toml`: package metadata and keywords.
- `README.md`: project description, examples, environment variables, and quality-gate notes.
- `src/main.rs`: crate path.
- `tests/cli_fuzzing.rs`: crate path.
- `src/lib.rs`: implementation repository URL.
- `src/cli/mod.rs`: top-level description, env var docs, and example commands.
- `src/paths/mod.rs`: `APP_HOME_*` and `APP_CACHE_*` placeholders.
- `run-profiler.ps1`: default traced command.

## Quality Bar
- The destination repository does not contain template-only files.
- The new crate name is used consistently in metadata, entrypoints, tests, and docs.
- No unintended `TODO(template)` markers remain.
- Help output describes the real tool rather than the demonstration commands.
- Home and cache directory naming uses project-specific values.
- The standard validation flow completes successfully.

## Common Pitfalls
- Forgetting to rename the crate path in both `src/main.rs` and `tests/cli_fuzzing.rs`.
- Leaving `APP_HOME_DIR` and `APP_CACHE_DIR` in place, which leaks template-specific environment variables into the new project.
- Assuming the helper copies `init-other-repo.ps1`; it intentionally does not.
- Assuming the helper overwrites destination `README.md` or `LICENSE`; it intentionally preserves existing ones.

## Example Prompt Starters
- `/initialize-from-teamy-rust-cli Create a new CLI in ../my-new-cli named meta-takeout with repo TeamDman/meta-takeout.`
- `/initialize-from-teamy-rust-cli Scaffold this repo from teamy-rust-cli and replace every template placeholder with project-specific values.`
- `/initialize-from-teamy-rust-cli Review this generated repo for leftover teamy-rust-cli branding and fix it.`
