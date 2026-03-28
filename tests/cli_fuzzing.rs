//! CLI fuzzing tests using figue's arbitrary helper assertions.

// TODO(template): rename the crate path below after copying the template into a real project.
use teamy_rust_cli::cli::Cli;

#[test]
fn fuzz_cli_args_consistency() {
    figue::assert_to_args_consistency::<Cli>(5000)
        .expect("figue helper consistency check should pass");
}

#[test]
fn fuzz_cli_args_roundtrip() {
    figue::assert_to_args_roundtrip::<Cli>(500).expect("figue helper roundtrip check should pass");
}
