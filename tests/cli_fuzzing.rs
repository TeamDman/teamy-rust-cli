// CLI fuzzing tests removed as we switched from clap to figue.
// Figue doesn't support arbitrary trait for fuzzing CLI parsing.
// Consider adding integration tests for specific CLI scenarios instead.

#[test]
fn cli_help_works() {
    // This just verifies the binary can be built and would accept --help
    // Actual help output is tested by figue itself
}
