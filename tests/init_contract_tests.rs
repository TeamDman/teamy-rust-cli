use std::fs;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

fn scratch() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "teamy-init-contract-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}

#[test]
fn dry_run_is_inert_and_generated_identity_is_consistent() {
    let root = scratch();
    let destination = root.join("new-tool");
    let plan = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args(["--log-filter", "off", "init"])
        .arg(&destination)
        .args([
            "--dry-run",
            "--name",
            "new-tool",
            "--repository-url",
            "https://github.com/example/new-tool",
        ])
        .output()
        .unwrap();
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    assert!(!destination.exists());
    let plan = String::from_utf8(plan.stdout).unwrap();
    assert!(plan.contains("\"dry_run\": true"));
    assert!(!plan.contains("init-other-repo.ps1"));
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args(["--log-filter", "off", "init"])
        .arg(&destination)
        .args([
            "--name",
            "new-tool",
            "--repository-url",
            "https://github.com/example/new-tool",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(destination.join("template_inventory.rs").is_file());
    assert!(
        destination
            .join("docs/command-completion-budgets.md")
            .is_file()
    );
    assert!(
        destination
            .join("resources/check-windows-manifest.ps1")
            .is_file()
    );
    assert!(destination.join("tests/windows_manifest.rs").is_file());
    let windows_manifest = fs::read_to_string(destination.join("resources/app.manifest")).unwrap();
    assert!(windows_manifest.contains("PerMonitorV2,PerMonitor"));
    assert!(windows_manifest.contains("true/pm"));
    assert!(windows_manifest.contains("Microsoft.Windows.Common-Controls"));
    let manifest = fs::read_to_string(destination.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"new-tool\""));
    assert!(manifest.contains("license = \"MPL-2.0\""));
    assert!(manifest.contains("https://github.com/example/new-tool"));
    assert!(
        fs::read_to_string(destination.join("src/main.rs"))
            .unwrap()
            .contains("new_tool::entrypoint()")
    );
    assert!(
        fs::read_to_string(destination.join("src/paths/mod.rs"))
            .unwrap()
            .contains("NEW_TOOL_HOME_DIR")
    );
    fs::write(destination.join("README.md"), "Existing README\n").unwrap();
    fs::write(destination.join("LICENSE"), "Existing LICENSE\n").unwrap();
    let force = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args(["--log-filter", "off", "init"])
        .arg(&destination)
        .arg("--force")
        .output()
        .unwrap();
    assert!(force.status.success());
    assert_eq!(
        fs::read_to_string(destination.join("README.md")).unwrap(),
        "Existing README\n"
    );
    assert_eq!(
        fs::read_to_string(destination.join("LICENSE")).unwrap(),
        "Existing LICENSE\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn name_without_destination_uses_the_invoking_directory() {
    let root = scratch();
    fs::create_dir_all(&root).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .current_dir(&root)
        .args(["--log-filter", "off", "init", "--name", "local-tool"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest = fs::read_to_string(root.join("local-tool/Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"local-tool\""));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn overlapping_paths_and_invalid_identity_fail_without_writes() {
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args([
            "--log-format",
            "ndjson",
            "init",
            env!("CARGO_MANIFEST_DIR"),
            "--force",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let root = scratch();
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args(["--log-filter", "off", "init"])
        .arg(&root)
        .args(["--name", "../invalid"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!root.exists());
}
