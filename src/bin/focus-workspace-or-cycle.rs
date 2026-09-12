use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, WorkspaceReferenceArg};
use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

const USAGE: &str = "Usage: focus-workspace-or-cycle <index> [prev]";

#[derive(Debug, PartialEq, Eq)]
enum Cli {
    Help,
    Run { target: u8, prev: bool },
}

fn parse_args(args: &[OsString]) -> Result<Cli, &'static str> {
    let (target, prev) = match args {
        [arg] if arg == "--help" || arg == "-h" => return Ok(Cli::Help),
        [target] => (target, false),
        [target, direction] if direction == "prev" => (target, true),
        [] => return Err("missing workspace index"),
        [_, _] => return Err("the optional second argument must be 'prev'"),
        _ => return Err("too many arguments"),
    };
    let target = target
        .to_str()
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|&value| value > 0)
        .ok_or("workspace index must be an integer from 1 to 255")?;
    Ok(Cli::Run { target, prev })
}

fn ask(socket: &mut Socket, req: Request) -> Result<Response, String> {
    socket
        .send(req)
        .map_err(|error| format!("failed to communicate with niri: {error}"))?
        .map_err(|error| format!("niri returned an error: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let (target, prev) = match parse_args(&args) {
        Ok(Cli::Help) => {
            println!(
                "{USAGE}\n\nFocus a workspace, or cycle its windows if already focused.\n\n\
                 index       Workspace index (1-255)\n\
                 prev        Cycle backwards instead of forwards\n\
                 -h, --help  Show this help"
            );
            return ExitCode::SUCCESS;
        }
        Ok(Cli::Run { target, prev }) => (target, prev),
        Err(error) => {
            eprintln!("error: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match run(target, prev) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(target: u8, prev: bool) -> Result<(), String> {
    let mut socket =
        Socket::connect().map_err(|error| format!("failed to connect to niri: {error}"))?;

    let Response::Workspaces(workspaces) = ask(&mut socket, Request::Workspaces)? else {
        return Err("niri returned an unexpected response to Workspaces".into());
    };

    let target_ws = workspaces
        .iter()
        .find(|w| w.idx == target)
        .ok_or_else(|| format!("workspace {target} not found"))?;
    let focused_ws = workspaces.iter().find(|w| w.is_focused);

    // Not on the target workspace yet — just switch, don't touch windows.
    if focused_ws.map(|w| w.id) != Some(target_ws.id) {
        ask(
            &mut socket,
            Request::Action(Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Index(target),
            }),
        )?;
        return Ok(());
    }

    // Already there — cycle windows within the workspace.
    let Response::Windows(mut windows) = ask(&mut socket, Request::Windows)? else {
        return Err("niri returned an unexpected response to Windows".into());
    };
    windows.retain(|w| w.workspace_id == Some(target_ws.id));
    windows.sort_by_key(|w| w.id);

    if windows.len() <= 1 {
        return Ok(());
    }

    let cur = windows.iter().position(|w| w.is_focused).unwrap_or(0);
    let next = if prev {
        (cur + windows.len() - 1) % windows.len()
    } else {
        (cur + 1) % windows.len()
    };

    ask(
        &mut socket,
        Request::Action(Action::FocusWindow {
            id: windows[next].id,
        }),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn parses_indices_and_direction() {
        for (args, target, prev) in [
            (vec!["1"], 1, false),
            (vec!["255"], 255, false),
            (vec!["42", "prev"], 42, true),
        ] {
            let args: Vec<_> = args.into_iter().map(OsString::from).collect();
            assert_eq!(parse_args(&args), Ok(Cli::Run { target, prev }));
        }
    }

    #[test]
    fn parses_help() {
        for arg in ["-h", "--help"] {
            assert_eq!(parse_args(&[arg.into()]), Ok(Cli::Help));
        }
    }

    #[test]
    fn rejects_invalid_arguments() {
        for args in [
            vec![],
            vec![""],
            vec!["0"],
            vec!["256"],
            vec!["-1"],
            vec!["+1"],
            vec!["1.5"],
            vec!["web"],
            vec!["--unknown"],
            vec!["1", "next"],
            vec!["1", "prev", "extra"],
            vec!["--help", "extra"],
        ] {
            let args: Vec<_> = args.into_iter().map(OsString::from).collect();
            assert!(parse_args(&args).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn rejects_non_unicode_arguments() {
        let invalid = OsString::from_vec(vec![0xff]);
        assert!(parse_args(std::slice::from_ref(&invalid)).is_err());
        assert!(parse_args(&["1".into(), invalid]).is_err());
    }
}
