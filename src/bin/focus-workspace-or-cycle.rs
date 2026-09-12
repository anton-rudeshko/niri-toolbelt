use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, WorkspaceReferenceArg};
use std::env;
use std::process::ExitCode;

fn ask(socket: &mut Socket, req: Request) -> Response {
    socket
        .send(req)
        .expect("failed to talk to niri socket")
        .expect("niri returned an error")
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(target) = args.next().and_then(|s| s.parse::<u8>().ok()) else {
        eprintln!("usage: niri-ws-cycle <workspace-idx> [prev]");
        return ExitCode::FAILURE;
    };
    let prev = args.next().as_deref() == Some("prev");

    let mut socket = Socket::connect().expect("niri not running / no NIRI_SOCKET");

    let Response::Workspaces(workspaces) = ask(&mut socket, Request::Workspaces) else {
        unreachable!()
    };

    let target_ws = workspaces
        .iter()
        .find(|w| w.idx == target)
        .unwrap_or_else(|| {
            eprintln!("workspace {target} not found");
            std::process::exit(1)
        });
    let focused_ws = workspaces.iter().find(|w| w.is_focused);

    // Not on the target workspace yet — just switch, don't touch windows.
    if focused_ws.map(|w| w.id) != Some(target_ws.id) {
        ask(
            &mut socket,
            Request::Action(Action::FocusWorkspace {
                reference: WorkspaceReferenceArg::Index(target),
            }),
        );
        return ExitCode::SUCCESS;
    }

    // Already there — cycle windows within the workspace.
    let Response::Windows(mut windows) = ask(&mut socket, Request::Windows) else {
        unreachable!()
    };
    windows.retain(|w| w.workspace_id == Some(target_ws.id));
    windows.sort_by_key(|w| w.id);

    if windows.len() <= 1 {
        return ExitCode::SUCCESS;
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
    );

    ExitCode::SUCCESS
}
