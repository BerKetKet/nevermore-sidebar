//! nevermore-core: Headless core library for Nevermore Wayland sidebar.
//! Zero GUI dependencies. Contains sidebar state machine, domain traits, and types.

pub mod state;
pub mod traits;
pub mod types;

pub use state::{SidebarEvent, SidebarState, SidebarStateMachine};
pub use traits::{AdapterError, EdgeAdapter, NotificationBackend, SessionBackend, TaskBackend};
pub use types::{BackgroundTask, NotificationItem, TaskState, TerminalSession, ThemeTokens};
