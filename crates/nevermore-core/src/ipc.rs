use crate::state::SidebarState;
use crate::types::{BackgroundTask, TerminalSession};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

/// Gets the effective user ID without external C dependencies.
fn get_current_uid() -> u32 {
    fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(1000)
}

/// Path to the Nevermore daemon IPC socket.
pub fn default_socket_path() -> PathBuf {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/nevermore-{}", get_current_uid())));
    runtime_dir.join("nevermore").join("nevermore.sock")
}

/// Commands sent to the Nevermore resident daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum IpcRequest {
    Toggle,
    Show {
        tab: Option<String>,
    },
    Hide,
    Pin,
    Status,
    ListSessions,
    CreateSession {
        name: String,
    },
    KillSession {
        id: String,
    },
    ListTasks,
    LaunchTask {
        command: String,
        working_dir: String,
    },
    GetTaskLogs {
        unit: String,
        line_count: usize,
    },
    StopTask {
        unit: String,
    },
    RaiseApp {
        desktop_entry: String,
    },
    ListThemes,
    SetTheme {
        name: String,
    },
}

/// Responses returned by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum IpcResponse {
    State {
        state: SidebarState,
        active_tab: String,
        is_visible: bool,
        takes_focus: bool,
    },
    Sessions(Vec<TerminalSession>),
    Tasks(Vec<BackgroundTask>),
    Logs(String),
    Themes {
        active: String,
        available: Vec<String>,
    },
    Ok,
    Error(String),
}

/// Helper client to send an IPC request to the running Nevermore daemon.
pub fn send_ipc_request(request: &IpcRequest) -> Result<IpcResponse, String> {
    send_ipc_request_to(&default_socket_path(), request)
}

/// Send an IPC request to a specific socket path (useful for testing).
pub fn send_ipc_request_to(
    socket_path: &std::path::Path,
    request: &IpcRequest,
) -> Result<IpcResponse, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| {
        format!(
            "Failed to connect to daemon at '{}': {}. Is Nevermore running?",
            socket_path.display(),
            e
        )
    })?;

    let mut json = serde_json::to_string(request).map_err(|e| e.to_string())?;
    json.push('\n');
    stream
        .write_all(json.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;

    if line.trim().is_empty() {
        return Err("Daemon closed connection without response".to_string());
    }

    let response: IpcResponse = serde_json::from_str(&line)
        .map_err(|e| format!("Failed to parse response: {} (raw: '{}')", e, line.trim()))?;

    Ok(response)
}

/// Helper to create and bind the IPC socket with 0700 permissions on its directory.
pub fn create_ipc_listener(socket_path: &std::path::Path) -> Result<UnixListener, std::io::Error> {
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }

    if socket_path.exists() {
        fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    let _ = fs::set_permissions(socket_path, fs::Permissions::from_mode(0o700));
    Ok(listener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_ipc_roundtrip() {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = env::temp_dir().join(format!("nevermore-test-{}", ts));
        fs::create_dir_all(&test_dir).unwrap();
        let sock_path = test_dir.join("nevermore.sock");

        let listener = create_ipc_listener(&sock_path).unwrap();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();

            let req: IpcRequest = serde_json::from_str(&line).unwrap();
            assert_eq!(req, IpcRequest::Status);

            let resp = IpcResponse::State {
                state: SidebarState::Hidden,
                active_tab: "terminal".to_string(),
                is_visible: false,
                takes_focus: false,
            };

            let mut out = serde_json::to_string(&resp).unwrap();
            out.push('\n');
            let mut writer = stream;
            writer.write_all(out.as_bytes()).unwrap();
        });

        let resp = send_ipc_request_to(&sock_path, &IpcRequest::Status).unwrap();
        assert_eq!(
            resp,
            IpcResponse::State {
                state: SidebarState::Hidden,
                active_tab: "terminal".to_string(),
                is_visible: false,
                takes_focus: false,
            }
        );

        handle.join().unwrap();
        let _ = fs::remove_file(&sock_path);
        let _ = fs::remove_dir(&test_dir);
    }

    #[test]
    fn test_ipc_theme_requests() {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = env::temp_dir().join(format!("nevermore-theme-test-{}", ts));
        fs::create_dir_all(&test_dir).unwrap();
        let sock_path = test_dir.join("nevermore.sock");

        let listener = create_ipc_listener(&sock_path).unwrap();

        let handle = thread::spawn(move || {
            // First request: ListThemes
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let req: IpcRequest = serde_json::from_str(&line).unwrap();
            assert_eq!(req, IpcRequest::ListThemes);

            let resp = IpcResponse::Themes {
                active: "nevermore-dark".to_string(),
                available: vec!["catppuccin-mocha".to_string(), "nevermore-dark".to_string()],
            };
            let mut out = serde_json::to_string(&resp).unwrap();
            out.push('\n');
            let mut writer = stream;
            writer.write_all(out.as_bytes()).unwrap();

            // Second request: SetTheme
            let (stream2, _) = listener.accept().unwrap();
            let mut reader2 = BufReader::new(&stream2);
            let mut line2 = String::new();
            reader2.read_line(&mut line2).unwrap();
            let req2: IpcRequest = serde_json::from_str(&line2).unwrap();
            assert_eq!(
                req2,
                IpcRequest::SetTheme {
                    name: "catppuccin-mocha".to_string()
                }
            );

            let resp2 = IpcResponse::Ok;
            let mut out2 = serde_json::to_string(&resp2).unwrap();
            out2.push('\n');
            let mut writer2 = stream2;
            writer2.write_all(out2.as_bytes()).unwrap();
        });

        let resp1 = send_ipc_request_to(&sock_path, &IpcRequest::ListThemes).unwrap();
        assert_eq!(
            resp1,
            IpcResponse::Themes {
                active: "nevermore-dark".to_string(),
                available: vec!["catppuccin-mocha".to_string(), "nevermore-dark".to_string()],
            }
        );

        let resp2 = send_ipc_request_to(
            &sock_path,
            &IpcRequest::SetTheme {
                name: "catppuccin-mocha".to_string(),
            },
        )
        .unwrap();
        assert_eq!(resp2, IpcResponse::Ok);

        handle.join().unwrap();
        let _ = fs::remove_file(&sock_path);
        let _ = fs::remove_dir(&test_dir);
    }
}
