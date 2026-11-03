use nevermore_core::ipc::{create_ipc_listener, default_socket_path, IpcRequest, IpcResponse};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;

pub struct IpcCommand {
    pub request: IpcRequest,
    pub responder: Sender<IpcResponse>,
}

/// Spawns the background Unix domain socket listener thread, delivering commands directly to the GLib main loop.
pub fn start_ipc_server(sender: async_channel::Sender<IpcCommand>) -> std::io::Result<()> {
    let socket_path = default_socket_path();
    let listener = create_ipc_listener(&socket_path)?;

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
                continue;
            }

            let request: IpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = IpcResponse::Error(format!("Invalid IPC JSON: {}", e));
                    if let Ok(mut json) = serde_json::to_string(&err_resp) {
                        json.push('\n');
                        let _ = stream.write_all(json.as_bytes());
                    }
                    continue;
                }
            };

            let (tx, rx) = channel();
            let cmd = IpcCommand {
                request,
                responder: tx,
            };

            if sender.send_blocking(cmd).is_err() {
                break;
            }

            // Wait for UI to process the command
            if let Ok(response) = rx.recv() {
                if let Ok(mut json) = serde_json::to_string(&response) {
                    json.push('\n');
                    let _ = stream.write_all(json.as_bytes());
                    let _ = stream.flush();
                }
            }
        }
    });

    Ok(())
}
