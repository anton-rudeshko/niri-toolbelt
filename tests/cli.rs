use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::process::Command;

fn command() -> Command {
    without_niri(env!("CARGO_BIN_EXE_focus-workspace-or-cycle"))
}

fn without_niri(binary: &str) -> Command {
    let mut command = Command::new(binary);
    command.env_remove("NIRI_SOCKET");
    command
}

#[test]
fn help_succeeds_without_niri() {
    for arg in ["-h", "--help"] {
        let output = command().arg(arg).output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.starts_with("Usage: focus-workspace-or-cycle <index> [prev]"));
        assert!(stdout.contains("--help"));
    }
}

#[test]
fn invalid_arguments_exit_before_connecting() {
    for args in [
        vec![],
        vec!["0"],
        vec!["256"],
        vec!["web"],
        vec!["1", "next"],
        vec!["1", "prev", "extra"],
    ] {
        let output = command().args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("error:"));
        assert!(stderr.contains("Usage: focus-workspace-or-cycle"));
        assert!(!stderr.contains("panicked"));
        assert!(!stderr.contains("NIRI_SOCKET"));
    }
}

#[test]
fn missing_socket_reports_a_runtime_error() {
    for args in [vec!["1"], vec!["1", "prev"]] {
        let output = command().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("error: failed to connect to niri:"));
        assert!(stderr.contains("NIRI_SOCKET is not set"));
        assert!(!stderr.contains("panicked"));
        assert!(!stderr.contains("Usage:"));
    }
}

#[test]
fn invalid_socket_path_reports_a_runtime_error() {
    let output = command().arg("1").env("NIRI_SOCKET", "").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("error: failed to connect to niri:"));
    assert!(!stderr.contains("panicked"));
    assert!(!stderr.contains("Usage:"));
}

const CLOSE_WINDOW_UNLESS: &str = env!("CARGO_BIN_EXE_close-window-unless");
const CLOSE_WINDOW_USAGE: &str = "Usage: close-window-unless [--] <app_id>...";

#[test]
fn close_window_unless_help_succeeds_without_niri() {
    for arg in ["-h", "--help"] {
        let output = without_niri(CLOSE_WINDOW_UNLESS).arg(arg).output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.starts_with(CLOSE_WINDOW_USAGE));
    }
}

#[test]
fn close_window_unless_rejects_invalid_arguments_before_connecting() {
    for args in [
        vec![],
        vec!["--".into()],
        vec!["--unknown".into()],
        vec!["--help".into(), "app".into()],
        vec!["app".into(), "-h".into()],
        vec!["--".into(), OsString::from_vec(vec![0xff])],
    ] {
        let output = without_niri(CLOSE_WINDOW_UNLESS)
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("error:"));
        assert!(stderr.contains(CLOSE_WINDOW_USAGE));
        assert!(!stderr.contains("panicked"));
        assert!(!stderr.contains("NIRI_SOCKET"));
    }
}

#[test]
fn close_window_unless_reports_connection_failures() {
    for socket in [None, Some("")] {
        let mut command = without_niri(CLOSE_WINDOW_UNLESS);
        if let Some(socket) = socket {
            command.env("NIRI_SOCKET", socket);
        }
        let output = command.arg("app").output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("error: failed to connect to niri:"));
        assert!(!stderr.contains("panicked"));
        assert!(!stderr.contains("Usage:"));
    }
}
