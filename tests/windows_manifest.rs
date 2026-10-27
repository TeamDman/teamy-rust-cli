#[cfg(windows)]
#[test]
fn built_executable_has_native_dpi_manifest() {
    let executable = std::path::PathBuf::from(env!("CARGO_BIN_EXE_teamy-rust-cli"));
    for executable in [executable, std::env::current_exe().unwrap()] {
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/check-windows-manifest.ps1"
            ))
            .arg("-Executable")
            .arg(executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "manifest check failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
