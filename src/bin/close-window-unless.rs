use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response};
use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

const USAGE: &str = "Usage: close-window-unless [--] <app_id>...";

#[derive(Debug, PartialEq, Eq)]
enum Cli {
    Help,
    Run { exceptions: Vec<String> },
}

fn parse_args(args: &[OsString]) -> Result<Cli, &'static str> {
    if matches!(args, [arg] if arg == "--help" || arg == "-h") {
        return Ok(Cli::Help);
    }

    let mut exceptions = Vec::new();
    let mut literal = false;
    for arg in args {
        let arg = arg.to_str().ok_or("app_id must be valid Unicode")?;
        if !literal && arg == "--" {
            literal = true;
        } else if !literal && arg.starts_with('-') {
            return Err("unknown option; use -- before app_id values starting with '-'");
        } else {
            exceptions.push(arg.to_owned());
        }
    }
    if exceptions.is_empty() {
        return Err("missing app_id exception");
    }
    Ok(Cli::Run { exceptions })
}

fn ask(socket: &mut Socket, req: Request) -> Result<Response, String> {
    socket
        .send(req)
        .map_err(|error| format!("failed to communicate with niri: {error}"))?
        .map_err(|error| format!("niri returned an error: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let exceptions = match parse_args(&args) {
        Ok(Cli::Help) => {
            println!(
                "{USAGE}\n\nClose the focused window unless its app_id is excluded.\n\n\
                 app_id      One or more exact, case-sensitive app_id exceptions\n\
                 --          Treat remaining arguments as literal app_id values\n\
                 -h, --help  Show this help\n\n\
                 No focused window or a matching app_id: succeed without closing.\n\
                 Otherwise close the queried window by ID, even if focus changes.\n\
                 Exit status: 0 success or help, 2 invalid arguments, 1 IPC error."
            );
            return ExitCode::SUCCESS;
        }
        Ok(Cli::Run { exceptions }) => exceptions,
        Err(error) => {
            eprintln!("error: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    match run(&exceptions) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(exceptions: &[String]) -> Result<(), String> {
    let mut socket =
        Socket::connect().map_err(|error| format!("failed to connect to niri: {error}"))?;

    let Response::FocusedWindow(window) = ask(&mut socket, Request::FocusedWindow)? else {
        return Err("niri returned an unexpected response to FocusedWindow".into());
    };
    let Some(window) = window else {
        return Ok(());
    };
    if window
        .app_id
        .as_ref()
        .is_some_and(|app_id| exceptions.contains(app_id))
    {
        return Ok(());
    }

    // Focus can change between requests, so always target the queried window.
    match ask(
        &mut socket,
        Request::Action(Action::CloseWindow {
            id: Some(window.id),
        }),
    )? {
        Response::Handled => Ok(()),
        _ => Err("niri returned an unexpected response to Action".into()),
    }
}
