//! nevermore-core: Headless core library for Nevermore Wayland sidebar.
//! Zero GUI dependencies. Contains sidebar state machine, domain traits, types,
//! IPC socket client/server, tmux session backend, systemd task backend, and KWin adapter.

pub mod ipc;
pub mod kwin;
pub mod state;
pub mod systemd;
pub mod tmux;
pub mod traits;
pub mod types;

pub use ipc::{
    create_ipc_listener, default_socket_path, send_ipc_request, send_ipc_request_to, IpcRequest,
    IpcResponse,
};
pub use kwin::KWinAdapter;
pub use state::{SidebarEvent, SidebarState, SidebarStateMachine};
pub use systemd::SystemdTaskBackend;
pub use tmux::TmuxBackend;
pub use traits::{
    AdapterError, EdgeAdapter, NotificationBackend, NotificationError, SessionBackend,
    SessionError, TaskBackend, TaskError, WindowRaiser,
};
pub use types::{
    is_valid_tab, validate_tab, BackgroundTask, NotificationItem, TaskState, TerminalSession,
    ThemeTokens, CANONICAL_TABS,
};
