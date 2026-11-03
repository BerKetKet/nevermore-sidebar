mod ipc_server;
mod nav_rail;
mod terminal_pane;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, EventControllerKey,
    EventControllerMotion, GestureClick, Label, Orientation, Stack,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use ipc_server::{new_command_queue, start_ipc_server};
use nav_rail::NavRail;
use nevermore_core::ipc::IpcResponse;
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::state::{SidebarEvent, SidebarState, SidebarStateMachine};
use nevermore_core::tmux::TmuxBackend;
use nevermore_core::traits::{EdgeAdapter, SessionBackend, TaskBackend};
use nevermore_core::SystemdTaskBackend;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use terminal_pane::TerminalPane;

const PANEL_WIDTH: i32 = 380;
const HOT_EDGE_WIDTH: i32 = 2;
const AUTO_HIDE_DELAY_MS: u32 = 250;

fn get_process_rss_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

fn main() {
    let app = Application::builder()
        .application_id("io.github.nevermore.sidebar")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        // Keep the application running resident in the background indefinitely
        std::mem::forget(app.hold());

        // 1. Load CSS
        let provider = CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        // 2. Build Layer-Shell Window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Nevermore Wayland Sidebar")
            .default_height(1080)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_exclusive_zone(0);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        // Start in Hidden mode (2px hot strip)
        window.set_size_request(HOT_EDGE_WIDTH, -1);

        // 3. State Machine & Domain Backends
        let state_machine = Arc::new(Mutex::new(SidebarStateMachine::new()));
        let tmux = TmuxBackend::new("nevermore");
        let systemd = SystemdTaskBackend::new("nevermore-task");
        let kwin = KWinAdapter::new();

        // 4. Construct Layout
        let main_box = GtkBox::new(Orientation::Horizontal, 0);

        // Nav Rail
        let content_stack = Stack::new();
        content_stack.set_hexpand(true);
        content_stack.set_vexpand(true);

        let c_stack = content_stack.clone();
        let sm_for_nav = state_machine.clone();
        let nav_rail = NavRail::new(move |tab_name| {
            sm_for_nav.lock().unwrap().set_active_tab(tab_name);
            c_stack.set_visible_child_name(tab_name);
        });

        // Content Area with Header
        let content_area = GtkBox::new(Orientation::Vertical, 0);
        content_area.set_hexpand(true);
        content_area.set_vexpand(true);

        // Header
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.add_css_class("sidebar-header");

        let title_lbl = Label::new(Some("NEVERMORE // TERMINAL"));
        title_lbl.add_css_class("header-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);

        let badge_state = Label::new(Some("HIDDEN"));
        badge_state.add_css_class("badge-state");

        let btn_pin = Button::with_label("📌 PIN");
        btn_pin.add_css_class("btn-header-pin");

        let btn_close = Button::with_label("✕");
        btn_close.add_css_class("btn-header-close");

        header.append(&title_lbl);
        header.append(&badge_state);
        header.append(&btn_pin);
        header.append(&btn_close);

        content_area.append(&header);
        content_area.append(&content_stack);

        // 5. Add Terminal Pane to Content Stack
        let terminal_pane = TerminalPane::new();
        content_stack.add_named(terminal_pane.widget(), Some("terminal"));

        // Placeholder pages for other 8 tabs
        for (tab_id, tab_label) in nav_rail::TABS {
            if *tab_id != "terminal" {
                let tab_box = GtkBox::new(Orientation::Vertical, 12);
                tab_box.set_hexpand(true);
                tab_box.set_vexpand(true);
                tab_box.set_valign(gtk4::Align::Center);
                tab_box.set_halign(gtk4::Align::Center);

                let placeholder =
                    Label::new(Some(&format!("NEVERMORE // {}\n(Phase 7 Tab)", tab_label)));
                placeholder.add_css_class("header-title");
                tab_box.append(&placeholder);
                content_stack.add_named(&tab_box, Some(tab_id));
            }
        }

        main_box.append(nav_rail.widget());
        main_box.append(&content_area);
        window.set_child(Some(&main_box));

        // Shared Context for Visual Updates
        struct UiContext {
            window: ApplicationWindow,
            state_machine: Arc<Mutex<SidebarStateMachine>>,
            badge_state: Label,
            btn_pin: Button,
            nav_rail: Rc<NavRail>,
            content_stack: Stack,
            title_lbl: Label,
            leave_source: RefCell<Option<glib::SourceId>>,
        }

        let ctx = Rc::new(UiContext {
            window: window.clone(),
            state_machine: state_machine.clone(),
            badge_state: badge_state.clone(),
            btn_pin: btn_pin.clone(),
            nav_rail: nav_rail.clone(),
            content_stack: content_stack.clone(),
            title_lbl: title_lbl.clone(),
            leave_source: RefCell::new(None),
        });

        // Helper to sync UI to current State Machine state
        let apply_state = {
            let ctx = ctx.clone();
            Rc::new(move || {
                let sm = ctx.state_machine.lock().unwrap();
                let state = sm.state();
                let active_tab = sm.active_tab().to_string();
                drop(sm);

                ctx.title_lbl
                    .set_label(&format!("NEVERMORE // {}", active_tab.to_uppercase()));
                ctx.content_stack.set_visible_child_name(&active_tab);
                ctx.nav_rail.set_active_tab(&active_tab);

                match state {
                    SidebarState::Hidden => {
                        ctx.window.set_size_request(HOT_EDGE_WIDTH, -1);
                        ctx.window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
                        ctx.badge_state.set_label("HIDDEN");
                        ctx.badge_state.remove_css_class("engaged");
                        ctx.badge_state.remove_css_class("pinned");
                        ctx.btn_pin.remove_css_class("pinned");
                    }
                    SidebarState::Peek => {
                        ctx.window.set_size_request(PANEL_WIDTH, -1);
                        ctx.window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
                        ctx.badge_state.set_label("PEEK");
                        ctx.badge_state.remove_css_class("engaged");
                        ctx.badge_state.remove_css_class("pinned");
                        ctx.btn_pin.remove_css_class("pinned");
                    }
                    SidebarState::Engaged => {
                        ctx.window.set_size_request(PANEL_WIDTH, -1);
                        ctx.window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
                        ctx.badge_state.set_label("ENGAGED");
                        ctx.badge_state.add_css_class("engaged");
                        ctx.badge_state.remove_css_class("pinned");
                        ctx.btn_pin.remove_css_class("pinned");
                    }
                    SidebarState::Pinned => {
                        ctx.window.set_size_request(PANEL_WIDTH, -1);
                        ctx.window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
                        ctx.badge_state.set_label("PINNED");
                        ctx.badge_state.remove_css_class("engaged");
                        ctx.badge_state.add_css_class("pinned");
                        ctx.btn_pin.add_css_class("pinned");
                    }
                }
            })
        };

        // Pin Button Handler
        let apply_pin = apply_state.clone();
        let ctx_pin = ctx.clone();
        btn_pin.connect_clicked(move |_| {
            ctx_pin
                .state_machine
                .lock()
                .unwrap()
                .transition(SidebarEvent::TogglePinned);
            apply_pin();
        });

        // Close Button Handler
        let apply_close = apply_state.clone();
        let ctx_close = ctx.clone();
        btn_close.connect_clicked(move |_| {
            ctx_close
                .state_machine
                .lock()
                .unwrap()
                .transition(SidebarEvent::CliHide);
            apply_close();
        });

        // Click inside to Engage
        let gesture = GestureClick::new();
        let apply_click = apply_state.clone();
        let ctx_click = ctx.clone();
        gesture.connect_pressed(move |_, _, _, _| {
            let mut sm = ctx_click.state_machine.lock().unwrap();
            if sm.state() != SidebarState::Pinned {
                sm.transition(SidebarEvent::PointerClickInside);
            }
            drop(sm);
            apply_click();
        });
        main_box.add_controller(gesture);

        // Motion Controller (Hot-Edge Hover and Pointer Leave)
        let motion = EventControllerMotion::new();
        let apply_motion = apply_state.clone();
        let ctx_motion = ctx.clone();

        motion.connect_enter(move |_, _, _| {
            if let Some(source) = ctx_motion.leave_source.borrow_mut().take() {
                source.remove();
            }

            let mut sm = ctx_motion.state_machine.lock().unwrap();
            if sm.state() == SidebarState::Hidden {
                sm.transition(SidebarEvent::PointerEnterEdge);
                drop(sm);
                apply_motion();
            }
        });

        let apply_leave = apply_state.clone();
        let ctx_leave = ctx.clone();
        motion.connect_leave(move |_| {
            let sm = ctx_leave.state_machine.lock().unwrap();
            let state = sm.state();
            drop(sm);

            if state == SidebarState::Pinned {
                return;
            }

            let ctx_timer = ctx_leave.clone();
            let apply_timer = apply_leave.clone();
            let source_id = glib::timeout_add_local_once(
                std::time::Duration::from_millis(AUTO_HIDE_DELAY_MS as u64),
                move || {
                    ctx_timer.leave_source.borrow_mut().take();
                    let mut sm = ctx_timer.state_machine.lock().unwrap();
                    if sm.state() != SidebarState::Pinned {
                        sm.transition(SidebarEvent::PointerLeavePanel);
                        drop(sm);
                        apply_timer();
                    }
                },
            );
            *ctx_leave.leave_source.borrow_mut() = Some(source_id);
        });

        main_box.add_controller(motion);

        // Key Controller (Escape key to hide or un-focus)
        let key_ctrl = EventControllerKey::new();
        let apply_key = apply_state.clone();
        let ctx_key = ctx.clone();

        key_ctrl.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                let mut sm = ctx_key.state_machine.lock().unwrap();
                sm.transition(SidebarEvent::EscapePressed);
                drop(sm);
                apply_key();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });

        window.add_controller(key_ctrl);

        // 6. Connect IPC Server Queue to Main Context Polling
        let cmd_queue = new_command_queue();
        let queue_clone = cmd_queue.clone();
        let apply_ipc = apply_state.clone();
        let ctx_ipc = ctx.clone();
        let tmux_ipc = tmux.clone();
        let systemd_ipc = systemd.clone();
        let kwin_ipc = kwin.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let mut q = queue_clone.lock().unwrap();
            for cmd in q.drain(..) {
                let start_t = Instant::now();
                let response = match cmd.request {
                    nevermore_core::ipc::IpcRequest::Toggle => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliToggle);
                        let tab = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        println!(
                            "[IPC] Toggle executed in {:.2} ms -> {:?}",
                            latency_ms, next
                        );
                        IpcResponse::State {
                            state: next,
                            active_tab: tab,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::Show { tab } => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliShow { tab });
                        let active = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        println!("[IPC] Show executed in {:.2} ms -> {:?}", latency_ms, next);
                        IpcResponse::State {
                            state: next,
                            active_tab: active,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::Hide => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliHide);
                        let active = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        println!("[IPC] Hide executed in {:.2} ms -> {:?}", latency_ms, next);
                        IpcResponse::State {
                            state: next,
                            active_tab: active,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::Pin => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliPin);
                        let active = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        IpcResponse::State {
                            state: next,
                            active_tab: active,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::Status => {
                        let sm = ctx_ipc.state_machine.lock().unwrap();
                        let state = sm.state();
                        let active = sm.active_tab().to_string();
                        IpcResponse::State {
                            state,
                            active_tab: active,
                            is_visible: state.is_visible(),
                            takes_focus: state.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::ListSessions => {
                        match tmux_ipc.list_sessions() {
                            Ok(sessions) => IpcResponse::Sessions(sessions),
                            Err(e) => IpcResponse::Error(e.to_string()),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::CreateSession { name } => {
                        match tmux_ipc.create_session(&name) {
                            Ok(_) => IpcResponse::Ok,
                            Err(e) => IpcResponse::Error(e.to_string()),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::KillSession { id } => {
                        match tmux_ipc.kill_session(&id) {
                            Ok(_) => IpcResponse::Ok,
                            Err(e) => IpcResponse::Error(e.to_string()),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::ListTasks => match systemd_ipc.list_tasks() {
                        Ok(tasks) => IpcResponse::Tasks(tasks),
                        Err(e) => IpcResponse::Error(e.to_string()),
                    },
                    nevermore_core::ipc::IpcRequest::LaunchTask {
                        command,
                        working_dir,
                    } => match systemd_ipc.launch_task(&command, &working_dir) {
                        Ok(_) => IpcResponse::Ok,
                        Err(e) => IpcResponse::Error(e.to_string()),
                    },
                    nevermore_core::ipc::IpcRequest::GetTaskLogs { unit, line_count } => {
                        match systemd_ipc.get_task_logs(&unit, line_count) {
                            Ok(logs) => IpcResponse::Logs(logs),
                            Err(e) => IpcResponse::Error(e.to_string()),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::StopTask { unit } => {
                        match systemd_ipc.stop_task(&unit) {
                            Ok(_) => IpcResponse::Ok,
                            Err(e) => IpcResponse::Error(e.to_string()),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::RaiseApp { desktop_entry } => {
                        match kwin_ipc.raise_application(&desktop_entry) {
                            Ok(_) => IpcResponse::Ok,
                            Err(e) => IpcResponse::Error(e.to_string()),
                        }
                    }
                };

                let _ = cmd.responder.send(response);
            }
            glib::ControlFlow::Continue
        });

        if let Err(e) = start_ipc_server(cmd_queue) {
            eprintln!("[ERROR] Failed to start IPC socket server: {}", e);
        } else {
            println!(
                "[IPC] Server successfully bound at {:?}",
                nevermore_core::ipc::default_socket_path()
            );
        }

        window.present();
        println!(
            "[NEVERMORE] UI initialized. Initial RSS: {:.2} MB",
            get_process_rss_mb()
        );
    });

    app.run();
}
