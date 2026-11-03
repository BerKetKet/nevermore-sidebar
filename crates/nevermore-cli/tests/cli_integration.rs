use nevermore_core::{create_ipc_listener, IpcRequest, IpcResponse, SidebarState};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_cli_ipc_integration() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let runtime_dir = env::temp_dir().join(format!("nevermore-cli-test-{}", ts));
    let sock_dir = runtime_dir.join("nevermore");
    fs::create_dir_all(&sock_dir).unwrap();
    let sock_path = sock_dir.join("nevermore.sock");

    let listener = create_ipc_listener(&sock_path).unwrap();

    let server_handle = thread::spawn(move || {
        // Handle 4 requests: status, toggle, show tasks, sessions
        for _ in 0..4 {
            if let Ok((stream, _)) = listener.accept() {
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    if let Ok(req) = serde_json::from_str::<IpcRequest>(&line) {
                        let resp = match req {
                            IpcRequest::Status => IpcResponse::State {
                                state: SidebarState::Hidden,
                                active_tab: "terminal".to_string(),
                                is_visible: false,
                                takes_focus: false,
                            },
                            IpcRequest::Toggle => IpcResponse::State {
                                state: SidebarState::Engaged,
                                active_tab: "terminal".to_string(),
                                is_visible: true,
                                takes_focus: true,
                            },
                            IpcRequest::Show { tab } => IpcResponse::State {
                                state: SidebarState::Engaged,
                                active_tab: tab.unwrap_or_else(|| "terminal".to_string()),
                                is_visible: true,
                                takes_focus: true,
                            },
                            IpcRequest::ListSessions => IpcResponse::Sessions(vec![]),
                            _ => IpcResponse::Ok,
                        };

                        let mut out = serde_json::to_string(&resp).unwrap();
                        out.push('\n');
                        let mut writer = stream;
                        let _ = writer.write_all(out.as_bytes());
                    }
                }
            }
        }
    });

    let bin_path = env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("nevermore");

    // 1. Status command
    let output = Command::new(&bin_path)
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .arg("status")
        .output()
        .expect("Failed to execute nevermore binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sidebar State: Hidden"));
    assert!(stdout.contains("Active Tab: terminal"));

    // 2. Toggle command
    let output = Command::new(&bin_path)
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .arg("toggle")
        .output()
        .expect("Failed to execute nevermore binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sidebar State: Engaged"));

    // 3. Show tab command
    let output = Command::new(&bin_path)
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .arg("show")
        .arg("tasks")
        .output()
        .expect("Failed to execute nevermore binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Active Tab: tasks"));

    // 4. Sessions command
    let output = Command::new(&bin_path)
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .arg("sessions")
        .output()
        .expect("Failed to execute nevermore binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No active tmux sessions."));

    server_handle.join().unwrap();
    let _ = fs::remove_file(&sock_path);
    let _ = fs::remove_dir_all(&runtime_dir);
}
