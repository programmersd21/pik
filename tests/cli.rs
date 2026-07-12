use std::process::{Command, Stdio};

fn pik_binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_pik"));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

#[test]
fn test_no_input_exits_with_error() {
    let output = pik_binary()
        .stdin(Stdio::null())
        .output()
        .expect("failed to execute pik");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("USAGE") || stderr.contains("usage"));
}

#[test]
fn test_empty_input_exits_with_error() {
    let mut child = pik_binary().spawn().expect("failed to spawn pik");

    // Close stdin immediately (empty input)
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait on pik");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_help_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_pik"))
        .arg("--help")
        .output()
        .expect("failed to execute pik --help");

    assert_eq!(output.status.code(), Some(0));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("USAGE") || stderr.contains("usage"));
    assert!(stderr.contains("--prompt"));
}

#[test]
fn test_version_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_pik"))
        .arg("--version")
        .output()
        .expect("failed to execute pik --version");

    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pik"));
    assert!(stdout.chars().any(|c| c.is_ascii_digit()));
}

#[test]
fn test_invalid_option() {
    let output = Command::new(env!("CARGO_BIN_EXE_pik"))
        .arg("--invalid-option")
        .output()
        .expect("failed to execute pik");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("unknown"));
}

#[test]
fn test_prompt_requires_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_pik"))
        .arg("--prompt")
        .output()
        .expect("failed to execute pik");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("prompt") || stderr.contains("value") || stderr.contains("requires"));
}

#[test]
fn test_file_requires_value() {
    let output = Command::new(env!("CARGO_BIN_EXE_pik"))
        .arg("--file")
        .output()
        .expect("failed to execute pik");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("file") || stderr.contains("value") || stderr.contains("requires"));
}

#[test]
fn test_file_not_found() {
    let output = Command::new(env!("CARGO_BIN_EXE_pik"))
        .arg("--file")
        .arg("nonexistent_file_12345.txt")
        .output()
        .expect("failed to execute pik");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error"));
}

// Note: Full interactive keyboard testing (arrow keys, Enter, Esc) requires
// a pty harness (e.g., rexpect or scripted expect-style testing) and is not
// practical in standard CI without additional infrastructure. The tests above
// cover the non-interactive paths: argument parsing, empty input handling,
// and exit code contracts that can be verified without terminal emulation.
