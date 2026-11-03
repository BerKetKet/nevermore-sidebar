mod ipc_server;
mod nav_rail;
mod terminal_pane;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, EventControllerKey,
    EventControllerMotion, EventControllerScroll, EventControllerScrollFlags, GestureClick, Label,
    Orientation, Stack,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use ipc_server::start_ipc_server;
use nav_rail::NavRail;
use nevermore_core::ipc::IpcResponse;
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::state::{SidebarEvent, SidebarState, SidebarStateMachine};
use nevermore_core::tmux::TmuxBackend;
use nevermore_core::traits::{SessionBackend, TaskBackend, WindowRaiser};
use nevermore_core::SystemdTaskBackend;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use terminal_pane::TerminalPane;

pub const PANEL_WIDTH: i32 = 400;
pub const NAV_RAIL_WIDTH: i32 = 60;
pub const CONTENT_WIDTH: i32 = 340;
pub const HOT_EDGE_WIDTH: i32 = 2;
pub const AUTO_HIDE_DELAY_MS: u32 = 250;

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

        // 2A. Build 2px Hot-Edge Layer-Shell Window (active only when Hidden)
        let hot_edge_window = ApplicationWindow::builder()
            .application(app)
            .title("Nevermore Hot Edge")
            .default_height(1080)
            .build();

        hot_edge_window.init_layer_shell();
        hot_edge_window.set_layer(Layer::Top);
        hot_edge_window.set_exclusive_zone(0);
        hot_edge_window.set_anchor(Edge::Right, true);
        hot_edge_window.set_anchor(Edge::Top, true);
        hot_edge_window.set_anchor(Edge::Bottom, true);
        hot_edge_window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
        hot_edge_window.set_size_request(HOT_EDGE_WIDTH, -1);

        let hot_edge_box = GtkBox::new(Orientation::Vertical, 0);
        hot_edge_box.add_css_class("hot-edge-strip");
        hot_edge_box.set_size_request(HOT_EDGE_WIDTH, -1);
        hot_edge_window.set_child(Some(&hot_edge_box));

        // 2B. Build 400px Main Sidebar Panel Window
        let panel_window = ApplicationWindow::builder()
            .application(app)
            .title("Nevermore Wayland Sidebar")
            .default_height(1080)
            .build();

        panel_window.init_layer_shell();
        panel_window.set_layer(Layer::Top);
        panel_window.set_exclusive_zone(0);
        panel_window.set_anchor(Edge::Right, true);
        panel_window.set_anchor(Edge::Top, true);
        panel_window.set_anchor(Edge::Bottom, true);
        panel_window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);

        // Lock window width strictly to 400px
        panel_window.set_size_request(PANEL_WIDTH, -1);
        panel_window.set_default_size(PANEL_WIDTH, -1);
        panel_window.set_resizable(false);
        panel_window.add_css_class("sidebar-panel");

        // 3. State Machine & Domain Backends
        let state_machine = Arc::new(Mutex::new(SidebarStateMachine::new()));
        let tmux = TmuxBackend::new("nevermore");
        let systemd = SystemdTaskBackend::new("nevermore-task");
        let kwin = KWinAdapter::new();

        // 4. Construct Layout locked to 400px (60px rail + 340px content)
        let main_box = GtkBox::new(Orientation::Horizontal, 0);
        main_box.set_size_request(PANEL_WIDTH, -1);
        main_box.set_hexpand(false);

        // Nav Rail
        let content_stack = Stack::new();
        content_stack.set_hexpand(true);
        content_stack.set_vexpand(true);

        let c_stack = content_stack.clone();
        let sm_for_nav = state_machine.clone();
        let nav_rail = NavRail::new(move |tab_name| {
            let _ = sm_for_nav.lock().unwrap().set_active_tab(tab_name);
            c_stack.set_visible_child_name(tab_name);
        });
        nav_rail.widget().set_size_request(NAV_RAIL_WIDTH, -1);
        nav_rail.widget().set_hexpand(false);

        // Content Area with Header
        let content_area = GtkBox::new(Orientation::Vertical, 0);
        content_area.set_size_request(CONTENT_WIDTH, -1);
        content_area.set_hexpand(false);
        content_area.set_vexpand(true);

        // Header
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.add_css_class("sidebar-header");

        let title_lbl = Label::new(Some("NEVERMORE // TERMINAL"));
        title_lbl.add_css_class("header-title");
        title_lbl.set_hexpand(true);
        title_lbl.set_xalign(0.0);
        title_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let badge_state = Label::new(Some("HIDDEN"));
        badge_state.add_css_class("badge-state");

        let btn_pin = Button::with_label("📌");
        btn_pin.set_tooltip_text(Some("Toggle Pin"));
        btn_pin.add_css_class("btn-header-pin");

        let btn_close = Button::with_label("✕");
        btn_close.set_tooltip_text(Some("Close Sidebar (Escape)"));
        btn_close.add_css_class("btn-header-close");

        header.append(&title_lbl);
        header.append(&badge_state);
        header.append(&btn_pin);
        header.append(&btn_close);

        // Attach horizontal swipe gesture controller to Header Bar (Fix 8 - Zone Rule)
        let header_scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::HORIZONTAL);
        let nav_for_header = nav_rail.clone();
        let c_stack_for_header = content_stack.clone();
        let sm_for_header = state_machine.clone();
        header_scroll_ctrl.connect_scroll(move |_, dx, _| {
            if dx.abs() > 0.5 {
                let cur = sm_for_header.lock().unwrap().active_tab().to_string();
                let current_idx = nav_rail::TABS
                    .iter()
                    .position(|(id, _, _)| *id == cur)
                    .unwrap_or(0);
                let next_idx = if dx > 0.0 {
                    (current_idx + 1) % nav_rail::TABS.len()
                } else if current_idx == 0 {
                    nav_rail::TABS.len() - 1
                } else {
                    current_idx - 1
                };
                let next_tab = nav_rail::TABS[next_idx].0;
                let _ = sm_for_header.lock().unwrap().set_active_tab(next_tab);
                nav_for_header.set_active_tab(next_tab);
                c_stack_for_header.set_visible_child_name(next_tab);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        header.add_controller(header_scroll_ctrl);

        content_area.append(&header);
        content_area.append(&content_stack);

        // 5. Add Terminal Pane to Content Stack
        let terminal_pane = TerminalPane::new();
        content_stack.add_named(terminal_pane.widget(), Some("terminal"));

        // Placeholder pages for other 8 tabs
        for (tab_id, _, tab_label) in nav_rail::TABS {
            if *tab_id != "terminal" {
                let tab_box = GtkBox::new(Orientation::Vertical, 12);
                tab_box.set_hexpand(true);
                tab_box.set_vexpand(true);
                tab_box.set_valign(gtk4::Align::Center);
                tab_box.set_halign(gtk4::Align::Center);

                let text = if *tab_id == "email" {
                    "NEVERMORE // EMAIL\nEmail Feasibility — Phase 11 Spike".to_string()
                } else {
                    format!("NEVERMORE // {}\n(Phase 7 Tab)", tab_label)
                };
                let placeholder = Label::new(Some(&text));
                placeholder.add_css_class("header-title");
                tab_box.append(&placeholder);
                content_stack.add_named(&tab_box, Some(tab_id));
            }
        }

        main_box.append(nav_rail.widget());
        main_box.append(&content_area);
        panel_window.set_child(Some(&main_box));

        // Shared Context for Visual Updates
        struct UiContext {
            hot_edge_window: ApplicationWindow,
            panel_window: ApplicationWindow,
            state_machine: Arc<Mutex<SidebarStateMachine>>,
            badge_state: Label,
            btn_pin: Button,
            nav_rail: Rc<NavRail>,
            content_stack: Stack,
            title_lbl: Label,
            leave_source: RefCell<Option<glib::SourceId>>,
        }

        let ctx = Rc::new(UiContext {
            hot_edge_window: hot_edge_window.clone(),
            panel_window: panel_window.clone(),
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
                        ctx.panel_window.set_visible(false);
                        ctx.hot_edge_window.set_visible(true);
                        ctx.badge_state.set_label("HIDDEN");
                        ctx.badge_state.remove_css_class("engaged");
                        ctx.badge_state.remove_css_class("pinned");
                        ctx.btn_pin.remove_css_class("pinned");
                    }
                    SidebarState::Peek => {
                        ctx.hot_edge_window.set_visible(false);
                        ctx.panel_window.set_visible(true);
                        ctx.panel_window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
                        ctx.badge_state.set_label("PEEK");
                        ctx.badge_state.remove_css_class("engaged");
                        ctx.badge_state.remove_css_class("pinned");
                        ctx.btn_pin.remove_css_class("pinned");
                    }
                    SidebarState::Engaged => {
                        ctx.hot_edge_window.set_visible(false);
                        ctx.panel_window.set_visible(true);
                        ctx.panel_window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
                        ctx.badge_state.set_label("ENGAGED");
                        ctx.badge_state.add_css_class("engaged");
                        ctx.badge_state.remove_css_class("pinned");
                        ctx.btn_pin.remove_css_class("pinned");
                    }
                    SidebarState::Pinned => {
                        ctx.hot_edge_window.set_visible(false);
                        ctx.panel_window.set_visible(true);
                        ctx.panel_window
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

        // Pointer Enter on Hot Edge Window (Hidden -> Peek)
        let edge_motion = EventControllerMotion::new();
        let apply_edge = apply_state.clone();
        let ctx_edge = ctx.clone();
        edge_motion.connect_enter(move |_, _, _| {
            ctx_edge
                .state_machine
                .lock()
                .unwrap()
                .transition(SidebarEvent::PointerEnterEdge);
            apply_edge();
        });
        hot_edge_window.add_controller(edge_motion);

        // Click inside Panel Window (Peek -> Engaged)
        let click_inside = GestureClick::new();
        let apply_click = apply_state.clone();
        let ctx_click = ctx.clone();
        click_inside.connect_pressed(move |_, _, _, _| {
            ctx_click
                .state_machine
                .lock()
                .unwrap()
                .transition(SidebarEvent::PointerClickInside);
            apply_click();
        });
        panel_window.add_controller(click_inside);

        // Motion Controller on Panel Window (Cancel debounce on enter, auto-hide on leave)
        let panel_motion = EventControllerMotion::new();
        let ctx_leave = ctx.clone();

        panel_motion.connect_enter(move |_, _, _| {
            if let Some(source_id) = ctx_leave.leave_source.borrow_mut().take() {
                source_id.remove();
            }
        });

        let ctx_timer_leave = ctx.clone();
        let apply_timer = apply_state.clone();
        panel_motion.connect_leave(move |_| {
            if let Some(source_id) = ctx_timer_leave.leave_source.borrow_mut().take() {
                source_id.remove();
            }

            let ctx_timer = ctx_timer_leave.clone();
            let apply_timer_cb = apply_timer.clone();

            let source_id = glib::timeout_add_local(
                std::time::Duration::from_millis(AUTO_HIDE_DELAY_MS as u64),
                move || {
                    ctx_timer.leave_source.borrow_mut().take();
                    let mut sm = ctx_timer.state_machine.lock().unwrap();
                    if sm.state() != SidebarState::Pinned {
                        sm.transition(SidebarEvent::PointerLeavePanel);
                        drop(sm);
                        apply_timer_cb();
                    }
                    glib::ControlFlow::Break
                },
            );
            *ctx_timer_leave.leave_source.borrow_mut() = Some(source_id);
        });
        panel_window.add_controller(panel_motion);

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
        panel_window.add_controller(key_ctrl);

        // 6. Connect IPC Server Channel (Zero-polling event-driven channel)
        let (ipc_tx, ipc_rx) = async_channel::unbounded::<ipc_server::IpcCommand>();
        let apply_ipc = apply_state.clone();
        let ctx_ipc = ctx.clone();
        let tmux_ipc = tmux.clone();
        let systemd_ipc = systemd.clone();
        let kwin_ipc = kwin.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = ipc_rx.recv().await {
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
                        if let Some(ref t) = tab {
                            if !nevermore_core::is_valid_tab(t) {
                                IpcResponse::Error(format!(
                                    "Invalid tab '{}'. Valid tabs are: {:?}",
                                    t,
                                    nevermore_core::CANONICAL_TABS
                                ))
                            } else {
                                let mut sm = ctx_ipc.state_machine.lock().unwrap();
                                let next = sm.transition(SidebarEvent::CliShow { tab });
                                let active = sm.active_tab().to_string();
                                drop(sm);
                                apply_ipc();
                                let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                                println!(
                                    "[IPC] Show executed in {:.2} ms -> {:?}",
                                    latency_ms, next
                                );
                                IpcResponse::State {
                                    state: next,
                                    active_tab: active,
                                    is_visible: next.is_visible(),
                                    takes_focus: next.takes_keyboard_focus(),
                                }
                            }
                        } else {
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
                    }
                    nevermore_core::ipc::IpcRequest::Hide => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliHide);
                        let tab = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        println!("[IPC] Hide executed in {:.2} ms -> {:?}", latency_ms, next);
                        IpcResponse::State {
                            state: next,
                            active_tab: tab,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::Pin => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliPin);
                        let tab = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        println!("[IPC] Pin executed in {:.2} ms -> {:?}", latency_ms, next);
                        IpcResponse::State {
                            state: next,
                            active_tab: tab,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::Status => {
                        let sm = ctx_ipc.state_machine.lock().unwrap();
                        IpcResponse::State {
                            state: sm.state(),
                            active_tab: sm.active_tab().to_string(),
                            is_visible: sm.state().is_visible(),
                            takes_focus: sm.state().takes_keyboard_focus(),
                        }
                    }
                    nevermore_core::ipc::IpcRequest::ListSessions => match tmux_ipc.list_sessions()
                    {
                        Ok(sessions) => IpcResponse::Sessions(sessions),
                        Err(e) => IpcResponse::Error(e.to_string()),
                    },
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
        });

        if let Err(e) = start_ipc_server(ipc_tx) {
            eprintln!("[ERROR] Failed to start IPC socket server: {}", e);
        } else {
            println!(
                "[IPC] Server successfully bound at {:?}",
                nevermore_core::ipc::default_socket_path()
            );
        }

        // Present initial UI in Hidden mode
        hot_edge_window.present();
        panel_window.set_visible(false);

        println!(
            "[NEVERMORE] UI initialized. Initial RSS: {:.2} MB",
            get_process_rss_mb()
        );
    });

    app.run();
}
