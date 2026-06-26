use std::process::Command;

#[test]
fn version_output_includes_git_repository_metadata() {
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .arg("--version")
        .output()
        .expect("failed to run --version");

    assert!(
        output.status.success(),
        "--version failed: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("--version output should be UTF-8");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "{stdout}");
    assert!(stdout.contains("repo "), "{stdout}");
    assert!(stdout.contains(env!("GIT_REPOSITORY_URL")), "{stdout}");
    assert!(stdout.contains("branch "), "{stdout}");
    assert!(stdout.contains(env!("GIT_BRANCH")), "{stdout}");
    assert!(stdout.contains("rev "), "{stdout}");
    assert!(stdout.contains(env!("GIT_REVISION")), "{stdout}");
    assert!(stdout.contains("worktree "), "{stdout}");
    assert!(stdout.contains(env!("GIT_WORKTREE_STATUS")), "{stdout}");
    assert!(stdout.contains("built "), "{stdout}");
}
