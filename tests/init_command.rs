use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[test]
fn init_scaffolds_template_and_refuses_implicit_overwrite() {
    let temp_root = unique_temp_dir("teamy-rust-cli-init-test");
    let destination = temp_root.join("generated-cli");
    let first = run_init(&destination);

    assert!(
        first.status.success(),
        "init failed: status={:?}\nstdout={}\nstderr={}",
        first.status.code(),
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(destination.join("Cargo.toml").is_file());
    assert!(destination.join("src").join("main.rs").is_file());
    assert!(destination.join("README.md").is_file());
    assert!(!destination.join(".git").exists());
    assert!(!destination.join("target").exists());
    assert!(!destination.join("init-other-repo.ps1").exists());
    assert!(
        !destination
            .join(".github")
            .join("skills")
            .join("initialize-from-teamy-rust-cli")
            .exists()
    );

    let second = run_init(&destination);
    assert!(
        !second.status.success(),
        "init should refuse overwriting generated files without --force"
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("--force"),
        "stderr should mention --force:\n{}",
        String::from_utf8_lossy(&second.stderr)
    );

    fs::remove_dir_all(temp_root).expect("failed to clean temp test directory");
}

fn run_init(destination: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args(["--output-format", "json", "init"])
        .arg(destination)
        .output()
        .expect("failed to run init")
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{name}-{}-{timestamp}", std::process::id()));
    if path.exists() {
        fs::remove_dir_all(&path).expect("failed to remove stale temp test directory");
    }
    path
}
