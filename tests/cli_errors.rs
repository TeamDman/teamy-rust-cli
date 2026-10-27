use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

const BINARY: &str = env!("CARGO_BIN_EXE_teamy-rust-cli");

#[derive(facet::Facet, Debug)]
struct FailureDiagnostic {
    level: String,
    message: String,
}

#[test]
fn logging_setup_errors_retain_their_cause_and_location_in_both_formats() {
    for format in ["text", "ndjson"] {
        let output = Command::new(BINARY)
            .args([
                "--log-format",
                format,
                "--log-file-filter",
                "fixture=bogus",
                "home",
                "show",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains('\u{1b}'));
        let message = if format == "ndjson" {
            assert_eq!(stderr.lines().count(), 1);
            let diagnostic: FailureDiagnostic = facet_json::from_str(stderr.trim()).unwrap();
            assert_eq!(diagnostic.level, "ERROR");
            diagnostic.message
        } else {
            stderr
        };
        assert!(message.contains("Cannot initialize logging"), "{message}");
        assert!(message.contains("invalid file log filter"), "{message}");
        assert!(message.contains("Location:"), "{message}");
    }
}

fn scratch() -> PathBuf {
    std::env::temp_dir().join(format!(
        "teamy-cli-boundary-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn malformed_commands_use_safe_diagnostics_without_opening_logs() {
    let root = scratch();
    let log = root.join("must-not-exist.ndjson");
    for args in [
        vec!["home", "show", "--log-filter"],
        vec!["home", "show", "--PRIVATE_SYNTHETIC_SECRET=value"],
        vec!["home", "show", "--log-format", "PRIVATE_SYNTHETIC_SECRET"],
    ] {
        let output = Command::new(BINARY)
            .args(["--log-format", "ndjson", "--log-file"])
            .arg(&log)
            .args(&args)
            .output()
            .unwrap();
        assert!(!output.status.success(), "unexpected success for {args:?}");
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.starts_with("ERROR: "));
        assert!(diagnostic.contains("[invalid-input]"));
        assert!(!diagnostic.contains("PRIVATE_SYNTHETIC_SECRET"));
        assert!(!root.exists());
    }
}

#[test]
fn builtins_and_incomplete_help_do_not_start_logging_or_echo_arguments() {
    let root = scratch();
    let log = root.join("must-not-exist.ndjson");
    for args in [
        vec!["--help"],
        vec!["cache", "show", "--help"],
        vec!["--version"],
        vec!["--log-filter", "SECRET_HELP_SUGGESTION"],
    ] {
        let output = Command::new(BINARY)
            .args(["--debug", "--log-file"])
            .arg(&log)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!root.exists());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stdout.contains("SECRET_HELP_SUGGESTION"));
        assert!(!stderr.contains("SECRET_HELP_SUGGESTION"));
        assert!(!stderr.contains("Tracing initialized"));
        assert!(!stderr.contains("Dispatching command"));
    }
}

#[cfg(windows)]
#[test]
fn non_unicode_input_fails_before_logging_without_exposing_code_units() {
    use std::os::windows::ffi::OsStringExt;
    let root = scratch();
    let argument = std::ffi::OsString::from_wide(&[0x0053, 0x0045, 0xd800, 0x0043]);
    let output = Command::new(BINARY)
        .args(["--debug", "--log-file"])
        .arg(root.join("must-not-exist.ndjson"))
        .args(["home", "show"])
        .arg(argument)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!root.exists());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("[invalid-input]"));
    assert!(!stderr.contains('\u{fffd}'));
    assert!(!stderr.contains("Process panicked"));
}
