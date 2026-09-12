use serde_json::{json, Value};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixListener;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn exchange(args: &[&str], replies: Vec<String>) -> (Output, Vec<Value>) {
    static NEXT_SOCKET: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "niri-toolbelt-{}-{}.sock",
        std::process::id(),
        NEXT_SOCKET.fetch_add(1, Ordering::Relaxed)
    ));
    let listener = UnixListener::bind(&path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "client did not connect");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut stream = BufReader::new(stream);
        let mut requests = Vec::new();
        for reply in replies {
            let mut line = String::new();
            assert!(stream.read_line(&mut line).unwrap() > 0);
            requests.push(serde_json::from_str(&line).unwrap());
            writeln!(stream.get_mut(), "{reply}").unwrap();
        }
        requests
    });
    let output = Command::new(env!("CARGO_BIN_EXE_focus-workspace-or-cycle"))
        .args(args)
        .env("NIRI_SOCKET", &path)
        .output()
        .unwrap();
    let requests = server.join().unwrap();
    std::fs::remove_file(path).unwrap();
    (output, requests)
}

fn workspaces(focused: bool) -> Value {
    json!({"Ok": {"Workspaces": [{
        "id": 10, "idx": 1, "name": null, "output": "test-output",
        "is_urgent": false, "is_active": focused, "is_focused": focused,
        "active_window_id": null
    }]}})
}

fn window(id: u64, position: [usize; 2], focused: bool, floating: bool) -> Value {
    json!({
        "id": id, "title": null, "app_id": null, "pid": null,
        "workspace_id": 10, "is_focused": focused, "is_floating": floating,
        "is_urgent": false, "focus_timestamp": null,
        "layout": {
            "pos_in_scrolling_layout": position,
            "tile_size": [100.0, 100.0], "window_size": [100, 100],
            "tile_pos_in_workspace_view": null, "window_offset_in_tile": [0.0, 0.0]
        }
    })
}

fn assert_success(output: &Output) {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

fn assert_error(output: &Output, message: &str) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(stderr.contains(message), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn switches_workspace_by_id_without_querying_windows() {
    let (output, requests) = exchange(
        &["1", "prev"],
        vec![
            workspaces(false).to_string(),
            json!({"Ok": "Handled"}).to_string(),
        ],
    );
    assert_success(&output);
    assert_eq!(
        requests,
        vec![
            json!("Workspaces"),
            json!({"Action": {"FocusWorkspace": {"reference": {"Id": 10}}}}),
        ]
    );
}

#[test]
fn sends_the_spatially_next_window_in_both_directions() {
    let windows = json!({"Ok": {"Windows": [
        window(5, [2, 1], false, false),
        window(80, [1, 2], true, false),
        window(30, [1, 1], false, false),
        window(1, [1, 3], false, true)
    ]}});
    for (args, target) in [(vec!["1"], 5), (vec!["1", "prev"], 30)] {
        let (output, requests) = exchange(
            &args,
            vec![
                workspaces(true).to_string(),
                windows.to_string(),
                json!({"Ok": "Handled"}).to_string(),
            ],
        );
        assert_success(&output);
        assert_eq!(
            requests,
            vec![
                json!("Workspaces"),
                json!("Windows"),
                json!({"Action": {"FocusWindow": {"id": target}}}),
            ]
        );
    }
}

#[test]
fn does_not_act_for_an_empty_or_already_focused_single_window() {
    for windows in [
        json!([]),
        json!([window(1, [1, 1], true, true)]),
        json!([window(1, [1, 1], true, false)]),
    ] {
        let (output, requests) = exchange(
            &["1"],
            vec![
                workspaces(true).to_string(),
                json!({"Ok": {"Windows": windows}}).to_string(),
            ],
        );
        assert_success(&output);
        assert_eq!(requests, vec![json!("Workspaces"), json!("Windows")]);
    }
}

#[test]
fn reports_missing_workspaces_and_server_errors() {
    for (reply, message) in [
        (json!({"Ok": {"Workspaces": []}}), "workspace 1 not found"),
        (
            json!({"Err": "test rejection"}),
            "niri returned an error: test rejection",
        ),
        (
            json!({"Ok": "Handled"}),
            "unexpected response to Workspaces",
        ),
    ] {
        let (output, requests) = exchange(&["1"], vec![reply.to_string()]);
        assert_error(&output, message);
        assert_eq!(requests, vec![json!("Workspaces")]);
    }
}

#[test]
fn reports_invalid_json_and_a_closed_connection() {
    let (output, _) = exchange(&["1"], vec!["not JSON".into()]);
    assert_error(&output, "failed to communicate with niri");
    let (output, _) = exchange(&["1"], vec![]);
    assert_error(&output, "failed to communicate with niri");
}

#[test]
fn reports_unexpected_window_and_action_responses() {
    let (output, _) = exchange(
        &["1"],
        vec![
            workspaces(true).to_string(),
            json!({"Ok": "Handled"}).to_string(),
        ],
    );
    assert_error(&output, "unexpected response to Windows");

    for (reply, message) in [
        (
            json!({"Ok": {"Windows": []}}),
            "unexpected response to Action",
        ),
        (
            json!({"Err": "window disappeared"}),
            "niri returned an error: window disappeared",
        ),
    ] {
        let (output, _) = exchange(
            &["1"],
            vec![workspaces(false).to_string(), reply.to_string()],
        );
        assert_error(&output, message);
    }
}
