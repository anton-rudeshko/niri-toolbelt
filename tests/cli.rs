use std::process::Command;

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_focus-workspace-or-cycle"));
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
