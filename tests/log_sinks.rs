use facet::Facet;
use std::fs;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Debug, Facet)]
struct LogLine {
    level: String,
}

fn scratch() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "teamy-log-test-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}

#[test]
fn independent_defaults_keep_command_lifecycle_in_debug_file() {
    let root = scratch();
    let file = root.join("capture.ndjson");
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args([
            "--log-format",
            "ndjson",
            "--output-format",
            "json",
            "--log-file",
        ])
        .arg(&file)
        .args(["home", "show"])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    for line in stderr.lines() {
        let record: LogLine = facet_json::from_str(line).unwrap();
        assert_ne!(record.level, "DEBUG");
    }
    let file_text = fs::read_to_string(&file).unwrap();
    assert!(
        file_text
            .lines()
            .map(|line| facet_json::from_str::<LogLine>(line).unwrap())
            .any(|record| record.level == "DEBUG")
    );
    assert!(file_text.contains("Dispatching command"));
    assert!(file_text.contains("Command finished"));
    let dispatch = file_text
        .lines()
        .find(|line| line.contains("Dispatching command"))
        .unwrap();
    assert!(dispatch.contains("\"level\":\"DEBUG\""));
    assert!(!stderr.contains("Dispatching command"));
    assert!(!stderr.contains("Command finished"));
    assert!(!file_text.contains('\u{1b}'));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("\"path\"")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stderr_off_does_not_disable_file_and_existing_files_are_appended() {
    let root = scratch();
    fs::create_dir_all(&root).unwrap();
    let file = root.join("capture.log");
    fs::write(&file, "prior record\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args([
            "--log-filter",
            "off",
            "--log-file-format",
            "text",
            "--log-file",
        ])
        .arg(&file)
        .args(["home", "show"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.starts_with("prior record\n"));
    assert!(text.contains("Tracing initialized"));
    assert!(!text.contains('\u{1b}'));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_filters_have_no_file_effects_and_emergency_json_omits_raw_payloads() {
    let root = scratch();
    let file = root.join("SYNTHETIC_SECRET_NEVER_LOG.ndjson");
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args([
            "--log-format",
            "ndjson",
            "--log-file-filter",
            "fixture=bogus",
            "--log-file",
        ])
        .arg(&file)
        .args(["home", "show"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!file.exists());
    let stderr = String::from_utf8(output.stderr).unwrap();
    for line in stderr.lines() {
        let _: LogLine = facet_json::from_str(line).unwrap();
    }
    assert!(!stderr.contains("SYNTHETIC_SECRET"));
}

#[test]
fn command_span_timings_reach_the_file_without_cluttering_stdio() {
    let root = scratch();
    let log = root.join("timings.ndjson");
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-rust-cli"))
        .args(["--log-filter", "info", "--log-file"])
        .arg(&log)
        .args(["--output-format", "json", "home", "show"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let ndjson = fs::read_to_string(&log).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"path\""));
    assert!(!stdout.contains("time.busy"));
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("time.busy")
    );
    let closes: Vec<_> = ndjson
        .lines()
        .filter(|line| line.contains("\"message\":\"close\""))
        .collect();
    assert_eq!(closes.len(), 1, "{ndjson}");
    assert!(closes[0].contains("\"name\":\"command\""));
    assert!(closes[0].contains("\"time.busy\""));
    assert!(closes[0].contains("\"time.idle\""));
    fs::remove_dir_all(root).unwrap();
}
