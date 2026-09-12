use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, Window, WorkspaceReferenceArg};
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

fn act(socket: &mut Socket, action: Action) -> Result<(), String> {
    match ask(socket, Request::Action(action))? {
        Response::Handled => Ok(()),
        _ => Err("niri returned an unexpected response to Action".into()),
    }
}

fn next_window(windows: &[Window], workspace_id: u64, prev: bool) -> Option<u64> {
    let mut tiled: Vec<_> = windows
        .iter()
        .filter(|window| window.workspace_id == Some(workspace_id) && !window.is_floating)
        .filter_map(|window| {
            window
                .layout
                .pos_in_scrolling_layout
                .map(|position| (position, window))
        })
        .collect();
    tiled.sort_unstable_by_key(|(position, _)| *position);

    if tiled.is_empty() {
        return None;
    }

    let current = tiled.iter().position(|(_, window)| window.is_focused);
    let next = match current {
        Some(_) if tiled.len() == 1 => return None,
        Some(index) if prev => (index + tiled.len() - 1) % tiled.len(),
        Some(index) => (index + 1) % tiled.len(),
        None if prev => tiled.len() - 1,
        None => 0,
    };
    Some(tiled[next].1.id)
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
    // Not on the target workspace yet — just switch, don't touch windows.
    if !target_ws.is_focused {
        act(
            &mut socket,
            Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Id(target_ws.id),
            },
        )?;
        return Ok(());
    }

    // Already there — cycle windows within the workspace.
    let Response::Windows(windows) = ask(&mut socket, Request::Windows)? else {
        return Err("niri returned an unexpected response to Windows".into());
    };
    if let Some(id) = next_window(&windows, target_ws.id, prev) {
        act(&mut socket, Action::FocusWindow { id })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use niri_ipc::WindowLayout;
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

    fn window(id: u64, position: (usize, usize), focused: bool) -> Window {
        Window {
            id,
            title: None,
            app_id: None,
            pid: None,
            workspace_id: Some(10),
            is_focused: focused,
            is_floating: false,
            is_urgent: false,
            layout: WindowLayout {
                pos_in_scrolling_layout: Some(position),
                tile_size: (100.0, 100.0),
                window_size: (100, 100),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
            focus_timestamp: None,
        }
    }

    #[test]
    fn cycles_in_spatial_order_in_both_directions() {
        // Neither IDs nor IPC order match the layout: left column, then right.
        let windows = [
            window(5, (2, 1), false),
            window(80, (1, 2), false),
            window(30, (1, 1), false),
        ];
        for (focused, forward, backward) in [(30, 80, 5), (80, 5, 30), (5, 30, 80)] {
            let mut windows = windows.clone();
            for window in &mut windows {
                window.is_focused = window.id == focused;
            }
            assert_eq!(next_window(&windows, 10, false), Some(forward));
            assert_eq!(next_window(&windows, 10, true), Some(backward));
        }
    }

    #[test]
    fn ignores_floating_and_unpositioned_windows_and_other_workspaces() {
        let mut floating = window(2, (1, 2), false);
        floating.is_floating = true;
        let mut other_workspace = window(3, (1, 3), false);
        other_workspace.workspace_id = Some(20);
        let mut no_workspace = window(4, (1, 4), false);
        no_workspace.workspace_id = None;
        let mut no_position = window(5, (1, 5), false);
        no_position.layout.pos_in_scrolling_layout = None;
        let windows = [
            window(1, (1, 1), true),
            floating,
            other_workspace,
            no_workspace,
            no_position,
            window(6, (2, 1), false),
        ];
        assert_eq!(next_window(&windows, 10, false), Some(6));
        assert_eq!(next_window(&windows, 10, true), Some(6));
    }

    #[test]
    fn enters_the_layout_from_either_end_without_a_focused_tiled_window() {
        let mut floating = window(2, (1, 2), true);
        floating.is_floating = true;
        for floating_focused in [false, true] {
            floating.is_focused = floating_focused;
            let windows = [
                window(1, (1, 1), false),
                floating.clone(),
                window(3, (2, 1), false),
            ];
            assert_eq!(next_window(&windows, 10, false), Some(1));
            assert_eq!(next_window(&windows, 10, true), Some(3));
        }
    }

    #[test]
    fn handles_empty_and_single_window_layouts() {
        let mut floating = window(2, (1, 1), true);
        floating.is_floating = true;
        for prev in [false, true] {
            assert_eq!(next_window(&[], 10, prev), None);
            assert_eq!(next_window(std::slice::from_ref(&floating), 10, prev), None);
            assert_eq!(next_window(&[window(1, (1, 1), true)], 10, prev), None);
            assert_eq!(next_window(&[window(1, (1, 1), false)], 10, prev), Some(1));
        }
    }
}
