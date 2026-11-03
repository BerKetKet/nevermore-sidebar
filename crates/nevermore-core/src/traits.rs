use crate::types::{BackgroundTask, NotificationItem, TerminalSession};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("Wayland layer-shell protocol error: {0}")]
    LayerShell(String),
    #[error("Compositor communication failure: {0}")]
    Compositor(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported compositor: {0}")]
    Unsupported(String),
}

/// Abstract compositor interface to isolate Wayland/KWin specific logic.
pub trait EdgeAdapter: Send + Sync {
    /// Configure right-edge wlr-layer-shell surface on TOP layer with zero strut.
    fn setup_layer_surface(&self, width: u32) -> Result<(), AdapterError>;

    /// Slide the panel in to reveal its content.
    fn show_panel(&self) -> Result<(), AdapterError>;

    /// Slide the panel out to hide its content.
    fn hide_panel(&self) -> Result<(), AdapterError>;

    /// Request physical keyboard focus from the compositor.
    fn request_keyboard_focus(&self) -> Result<(), AdapterError>;

    /// Release physical keyboard focus back to the underlying desktop window.
    fn release_keyboard_focus(&self) -> Result<(), AdapterError>;

    /// Activate or raise an external application window by desktop entry name.
    fn raise_application(&self, desktop_entry: &str) -> Result<(), AdapterError>;
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to connect to tmux server on socket '{socket}': {message}")]
    Connection { socket: String, message: String },
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Failed to execute command: {0}")]
    CommandFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Interface for durable terminal session management (e.g. backed by tmux).
pub trait SessionBackend: Send + Sync {
    fn list_sessions(&self) -> Result<Vec<TerminalSession>, SessionError>;
    fn create_session(&self, name: &str) -> Result<TerminalSession, SessionError>;
    fn kill_session(&self, id: &str) -> Result<(), SessionError>;
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("systemd-run invocation failed: {0}")]
    LaunchFailed(String),
    #[error("systemctl query failed: {0}")]
    QueryFailed(String),
    #[error("Invalid unit identifier: {0}")]
    InvalidUnit(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Interface for durable background task operations (backed by systemd-run --user).
pub trait TaskBackend: Send + Sync {
    fn list_tasks(&self) -> Result<Vec<BackgroundTask>, TaskError>;
    fn launch_task(&self, command: &str, working_dir: &str) -> Result<BackgroundTask, TaskError>;
    fn get_task_logs(&self, unit: &str, line_count: usize) -> Result<String, TaskError>;
    fn stop_task(&self, unit: &str) -> Result<(), TaskError>;
    fn dismiss_task(&self, unit: &str) -> Result<(), TaskError>;
}

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("D-Bus notification daemon error: {0}")]
    Dbus(String),
    #[error("Notification not found: {0}")]
    NotFound(u32),
}

/// Read-only inspection and client-action interface for system notifications.
/// Note: Never replaces or intercepts the desktop notification daemon.
pub trait NotificationBackend: Send + Sync {
    fn list_notifications(&self) -> Result<Vec<NotificationItem>, NotificationError>;
    fn invoke_action(&self, id: u32, action_key: &str) -> Result<(), NotificationError>;
    fn close_notification(&self, id: u32) -> Result<(), NotificationError>;
}
