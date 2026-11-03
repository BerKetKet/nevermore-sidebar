use crate::traits::{SessionBackend, SessionError};
use crate::types::TerminalSession;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Default tmux configuration content extracted and verified from legacy lessons.
pub const DEFAULT_TMUX_CONF: &str = r#"# Nevermore Sidebar tmux configuration
set -s escape-time 0
set -g mouse on
set -g history-limit 50000
set -g default-terminal "tmux-256color"
set -as terminal-features ",xterm-256color:RGB"
set -g status off
set -g prefix None
"#;

/// Session backend implementation communicating with a dedicated tmux server on a private socket.
#[derive(Debug, Clone)]
pub struct TmuxBackend {
    socket_name: String,
    conf_path: PathBuf,
}

impl Default for TmuxBackend {
    fn default() -> Self {
        Self::new("nevermore")
    }
}

impl TmuxBackend {
    pub fn new(socket_name: impl Into<String>) -> Self {
        let config_dir = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = env::var_os("HOME").unwrap_or_default();
                PathBuf::from(home).join(".config")
            });
        let conf_path = config_dir.join("nevermore").join("nevermore-tmux.conf");

        let backend = Self {
            socket_name: socket_name.into(),
            conf_path,
        };
        let _ = backend.ensure_config_file();
        backend
    }

    pub fn with_custom_conf(socket_name: impl Into<String>, conf_path: impl AsRef<Path>) -> Self {
        Self {
            socket_name: socket_name.into(),
            conf_path: conf_path.as_ref().to_path_buf(),
        }
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    pub fn conf_path(&self) -> &Path {
        &self.conf_path
    }

    /// Ensures the dedicated tmux configuration file exists on disk.
    pub fn ensure_config_file(&self) -> std::io::Result<()> {
        if let Some(parent) = self.conf_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !self.conf_path.exists() {
            fs::write(&self.conf_path, DEFAULT_TMUX_CONF)?;
        }
        Ok(())
    }

    fn tmux_cmd(&self) -> Command {
        let mut cmd = Command::new("tmux");
        cmd.arg("-L").arg(&self.socket_name);
        if self.conf_path.exists() {
            cmd.arg("-f").arg(&self.conf_path);
        }
        cmd
    }
}

impl SessionBackend for TmuxBackend {
    fn list_sessions(&self) -> Result<Vec<TerminalSession>, SessionError> {
        let output = self
            .tmux_cmd()
            .arg("list-sessions")
            .arg("-F")
            .arg("#{session_id}:#{session_name}:#{session_attached}:#{session_windows}:#{session_created}")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no server running")
                || stderr.contains("error connecting")
                || stderr.contains("no sessions")
            {
                return Ok(Vec::new());
            }
            return Err(SessionError::CommandFailed(format!(
                "tmux list-sessions failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut sessions = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 5 {
                let id = parts[0].to_string();
                let name = parts[1].to_string();
                let attached_clients = parts[2].parse().unwrap_or(0);
                let windows_count = parts[3].parse().unwrap_or(1);
                let created_at_unix = parts[4].parse().unwrap_or(0);

                sessions.push(TerminalSession {
                    id,
                    name,
                    attached_clients,
                    windows_count,
                    created_at_unix,
                });
            }
        }

        Ok(sessions)
    }

    fn create_session(&self, name: &str) -> Result<TerminalSession, SessionError> {
        let output = self
            .tmux_cmd()
            .arg("new-session")
            .arg("-d")
            .arg("-s")
            .arg(name)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SessionError::CommandFailed(format!(
                "Failed to create tmux session '{}': {}",
                name,
                stderr.trim()
            )));
        }

        // Fetch details of created session
        let sessions = self.list_sessions()?;
        sessions
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| SessionError::NotFound(name.to_string()))
    }

    fn kill_session(&self, id: &str) -> Result<(), SessionError> {
        let output = self
            .tmux_cmd()
            .arg("kill-session")
            .arg("-t")
            .arg(id)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SessionError::CommandFailed(format!(
                "Failed to kill tmux session '{}': {}",
                id,
                stderr.trim()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmux_session_lifecycle() {
        let test_sock = format!("test_tmux_{}", std::process::id());
        let backend = TmuxBackend::new(test_sock);

        // Initially no sessions
        let sessions = backend.list_sessions().expect("list_sessions failed");
        assert!(sessions.is_empty() || sessions.iter().all(|s| s.name != "unit_test_session"));

        // Create session
        let created = backend
            .create_session("unit_test_session")
            .expect("create_session failed");
        assert_eq!(created.name, "unit_test_session");

        // Verify session is listed
        let list_after = backend.list_sessions().expect("list_sessions failed");
        assert!(list_after.iter().any(|s| s.name == "unit_test_session"));

        // Kill session
        backend
            .kill_session("unit_test_session")
            .expect("kill_session failed");

        // Verify killed
        let list_final = backend.list_sessions().expect("list_sessions failed");
        assert!(!list_final.iter().any(|s| s.name == "unit_test_session"));
    }
}
