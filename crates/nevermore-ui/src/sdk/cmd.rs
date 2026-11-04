use std::collections::HashMap;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
    pub stdin_bytes: Option<Vec<u8>>,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct CmdResult {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl Cmd {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            envs: HashMap::new(),
            stdin_bytes: None,
            timeout: Duration::from_secs(2),
        }
    }

    pub fn curl(url: &str) -> Self {
        Self::new("curl")
            .args([
                "-fsS",
                "--max-time",
                "15",
                "--proto",
                "=https",
                "-A",
                "Nevermore/0.1",
                url,
            ])
            .timeout(Duration::from_secs(15))
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for a in args {
            self.args.push(a.into());
        }
        self
    }

    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.envs.insert(key.into(), val.into());
        self
    }

    pub fn stdin_bytes(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.stdin_bytes = Some(bytes.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Executes the command synchronously on the calling thread.
    /// Spawns in its own process group and enforces the timeout,
    /// killing the entire process group with SIGKILL if timed out.
    pub fn execute_blocking(&self) -> CmdResult {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        for (k, v) in &self.envs {
            command.env(k, v);
        }

        // Put child in its own process group
        command.process_group(0);
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }

        if self.stdin_bytes.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                return CmdResult {
                    code: None,
                    stdout: String::new(),
                    stderr: format!("Failed to spawn {}: {}", self.program, e),
                    timed_out: false,
                };
            }
        };

        if let Some(bytes) = &self.stdin_bytes {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(bytes);
            }
        }

        let pid = child.id() as i32;
        let timeout = self.timeout;
        let timed_out = Arc::new(AtomicBool::new(false));
        let timed_out_clone = timed_out.clone();

        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let watchdog = std::thread::spawn(move || {
            if done_rx.recv_timeout(timeout).is_err() {
                timed_out_clone.store(true, Ordering::SeqCst);
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
        });

        let output_res = child.wait_with_output();
        let _ = done_tx.send(());
        let _ = watchdog.join();

        let was_timed_out = timed_out.load(Ordering::SeqCst);

        match output_res {
            Ok(output) => CmdResult {
                code: if was_timed_out {
                    None
                } else {
                    output.status.code()
                },
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: if was_timed_out {
                    format!("Command '{}' timed out after {:?}", self.program, timeout)
                } else {
                    String::from_utf8_lossy(&output.stderr).to_string()
                },
                timed_out: was_timed_out,
            },
            Err(e) => CmdResult {
                code: None,
                stdout: String::new(),
                stderr: format!("Error waiting on child: {}", e),
                timed_out: was_timed_out,
            },
        }
    }

    /// Spawns the command detached in its own process group with PR_SET_PDEATHSIG.
    pub fn spawn_detached(&self) -> std::io::Result<()> {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        for (k, v) in &self.envs {
            command.env(k, v);
        }
        command.process_group(0);
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
        command.spawn().map(|_| ())
    }
}
