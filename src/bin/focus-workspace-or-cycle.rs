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
        eprintln!("usage: niri-ws-cycle <workspace-idx>");
        return ExitCode::FAILURE;
    };

    let mut socket = Socket::connect().expect("niri not running / no NIRI_SOCKET");

    let Response::Workspaces(workspaces) = ask(&mut socket, Request::Workspaces) else {
        unreachable!()
    };

    if !workspaces.iter().any(|w| w.idx == target) {
        eprintln!("workspace {target} not found");
        return ExitCode::FAILURE;
    }

    ask(
        &mut socket,
        Request::Action(Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Index(target),
        }),
    );

    ExitCode::SUCCESS
}
