//! CLI fuzzing tests using arbitrary-generated CLI instances.

use arbitrary::Arbitrary;
use teamy_rust_cli::cli::Cli;
use teamy_rust_cli::cli::ToArgs;

#[test]
fn fuzz_cli_args_consistency() {
    // Test that the same CLI instance always produces the same args
    let mut data = vec![123u8; 1024];
    let mut rng = arbitrary::Unstructured::new(&data);

    for i in 0..50 {
        let cli = match Cli::arbitrary(&mut rng) {
            Ok(cli) => cli,
            Err(_) => {
                data = vec![(i * 2) as u8; 1024];
                rng = arbitrary::Unstructured::new(&data);
                Cli::arbitrary(&mut rng).expect("Failed to generate CLI instance")
            }
        };

        let args1 = cli.to_args();
        let args2 = cli.to_args();

        assert_eq!(
            args1, args2,
            "CLI.to_args() should be deterministic for iteration {i}",
        );
    }
}

#[test]
fn cli_arbitrary_generates_valid_instances() {
    // Test that arbitrary generates valid CLI instances
    let data = vec![42u8; 1024];
    let mut rng = arbitrary::Unstructured::new(&data);

    let cli = Cli::arbitrary(&mut rng).expect("Failed to generate CLI instance");
    let args = cli.to_args();

    // The args should at least contain a subcommand
    assert!(
        !args.is_empty(),
        "CLI should generate at least a subcommand"
    );
}
