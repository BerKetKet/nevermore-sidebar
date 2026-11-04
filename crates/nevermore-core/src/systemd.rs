use crate::traits::{TaskBackend, TaskError};
use crate::types::{BackgroundTask, TaskState};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Task backend implementing transient systemd user service management.
#[derive(Debug, Clone)]
pub struct SystemdTaskBackend {
    unit_prefix: String,
}

impl Default for SystemdTaskBackend {
    fn default() -> Self {
        Self::new("nevermore-task")
    }
}

impl SystemdTaskBackend {
    pub fn new(unit_prefix: impl Into<String>) -> Self {
        Self {
            unit_prefix: unit_prefix.into(),
        }
    }

    pub fn unit_prefix(&self) -> &str {
        &self.unit_prefix
    }
}

impl TaskBackend for SystemdTaskBackend {
    fn list_tasks(&self) -> Result<Vec<BackgroundTask>, TaskError> {
        let pattern = format!("{}-*", self.unit_prefix);
        let output = Command::new("systemctl")
            .arg("--user")
            .arg("list-units")
            .arg(&pattern)
            .arg("--all")
            .arg("--no-legend")
            .arg("--no-pager")
            .arg("--plain")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskError::QueryFailed(stderr.trim().to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tasks = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let unit = parts[0].to_string();
                let active_state = parts[2];
                let sub_state = parts[3];

                let state = match (active_state, sub_state) {
                    ("active", "running") => TaskState::Running,
                    ("failed", _) => TaskState::Failed(1),
                    ("inactive", "dead") => TaskState::Exited(0),
                    _ => TaskState::Unknown,
                };

                let desc = parts[4..].join(" ");

                tasks.push(BackgroundTask {
                    unit,
                    command: desc,
                    working_dir: "".to_string(),
                    state,
                    exit_status: None,
                    timestamp_unix: 0,
                });
            }
        }

        Ok(tasks)
    }

    fn launch_task(&self, command: &str, working_dir: &str) -> Result<BackgroundTask, TaskError> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let unit_name = format!("{}-{}", self.unit_prefix, ts);

        let mut cmd = Command::new("systemd-run");
        cmd.arg("--user")
            .arg(format!("--unit={}", unit_name))
            .arg(format!("--description=Nevermore: {}", command));

        if !working_dir.is_empty() {
            cmd.arg(format!("--working-directory={}", working_dir));
        }

        cmd.arg("/bin/sh").arg("-c").arg(command);

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskError::LaunchFailed(stderr.trim().to_string()));
        }

        Ok(BackgroundTask {
            unit: format!("{}.service", unit_name),
            command: command.to_string(),
            working_dir: working_dir.to_string(),
            state: TaskState::Running,
            exit_status: None,
            timestamp_unix: (ts / 1000) as u64,
        })
    }

    fn get_task_logs(&self, unit: &str, line_count: usize) -> Result<String, TaskError> {
        let output = Command::new("journalctl")
            .arg("--user-unit")
            .arg(unit)
            .arg("-n")
            .arg(line_count.to_string())
            .arg("--no-pager")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskError::QueryFailed(stderr.trim().to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn stop_task(&self, unit: &str) -> Result<(), TaskError> {
        let output = Command::new("systemctl")
            .arg("--user")
            .arg("stop")
            .arg(unit)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskError::QueryFailed(stderr.trim().to_string()));
        }

        Ok(())
    }

    fn dismiss_task(&self, unit: &str) -> Result<(), TaskError> {
        let output = Command::new("systemctl")
            .arg("--user")
            .arg("reset-failed")
            .arg(unit)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TaskError::QueryFailed(stderr.trim().to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_systemd_task_launch_and_inspect() {
        if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
            eprintln!("Skipping systemd user bus test: no user D-Bus session");
            return;
        }
        let check = std::process::Command::new("systemctl")
            .arg("--user")
            .arg("status")
            .output();
        if check.is_err() || !check.unwrap().status.success() {
            eprintln!("Skipping systemd user bus test: systemctl --user unavailable");
            return;
        }

        let backend = SystemdTaskBackend::new("test-nevermore-unit");
        let task = backend
            .launch_task("echo 'Hello Nevermore Task'", "/tmp")
            .expect("launch_task failed");

        assert!(task.unit.contains("test-nevermore-unit"));

        // Wait a moment for execution
        thread::sleep(Duration::from_millis(500));

        let logs = backend
            .get_task_logs(&task.unit, 10)
            .expect("get_task_logs failed");
        assert!(logs.contains("Hello Nevermore Task") || logs.contains("Started"));

        let _ = backend.dismiss_task(&task.unit);
    }
}
