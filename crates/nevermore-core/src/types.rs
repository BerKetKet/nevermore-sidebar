use serde::{Deserialize, Serialize};

/// Represents an active or attached terminal session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub name: String,
    pub attached_clients: u32,
    pub windows_count: u32,
    pub created_at_unix: u64,
}

/// Execution state of a background transient task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Running,
    Exited(i32),
    Failed(i32),
    Unknown,
}

/// Represents a background transient task managed via systemd user services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub unit: String,
    pub command: String,
    pub working_dir: String,
    pub state: TaskState,
    pub exit_status: Option<i32>,
    pub timestamp_unix: u64,
}

/// Represents an inspected system notification card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationItem {
    pub id: u32,
    pub app_name: String,
    pub desktop_entry: Option<String>,
    pub summary: String,
    pub body: String,
    pub has_default_action: bool,
    pub actions: Vec<(String, String)>,
    pub timestamp_unix: u64,
}

/// Core theme palette tokens enforcing solid dark surfaces and blue accents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub surface_background: &'static str,
    pub surface_card: &'static str,
    pub surface_card_hover: &'static str,
    pub accent_primary: &'static str,
    pub accent_destructive: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub border_subtle: &'static str,
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self {
            surface_background: "#18181b",
            surface_card: "#202024",
            surface_card_hover: "#2a2a2f",
            accent_primary: "#0a84ff",
            accent_destructive: "#ff453a",
            text_primary: "#ffffff",
            text_secondary: "rgba(255, 255, 255, 0.6)",
            border_subtle: "rgba(255, 255, 255, 0.08)",
        }
    }
}

/// The 9 canonical first-release tab identifiers for Nevermore.
pub const CANONICAL_TABS: &[&str] = &[
    "terminal",
    "widgets",
    "notifications",
    "notepad",
    "media",
    "gemini",
    "youtube",
    "vpn",
    "email",
];

/// Validates whether a given string matches one of the canonical 9 tabs.
pub fn is_valid_tab(tab: &str) -> bool {
    CANONICAL_TABS.contains(&tab)
}

/// Normalizes or validates a tab identifier, returning a canonical reference if valid.
pub fn validate_tab(tab: &str) -> Result<&'static str, String> {
    for &t in CANONICAL_TABS {
        if t.eq_ignore_ascii_case(tab) {
            return Ok(t);
        }
    }
    Err(format!(
        "Invalid tab '{}'. Valid tabs are: {:?}",
        tab, CANONICAL_TABS
    ))
}
