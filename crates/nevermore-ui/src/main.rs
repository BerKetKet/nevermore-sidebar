use nevermore_ui::safe_source_remove;
use nevermore_ui::{icons, ipc_server, nav_rail, plugins};

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, EventControllerKey,
    EventControllerMotion, EventControllerScroll, EventControllerScrollFlags, GestureClick,
    Orientation, Stack,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use ipc_server::start_ipc_server;
use nav_rail::NavRail;
use nevermore_core::ipc::IpcResponse;
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::state::{SidebarEvent, SidebarState, SidebarStateMachine};
use nevermore_core::theme::{
    contrast_text_color_for_hex, custom_css_path, get_active_theme_name, get_appearance_config,
    get_disabled_plugins, install_bundled_themes, list_available_themes, load_theme,
    set_active_theme_name, ThemePalette,
};
use nevermore_core::tmux::TmuxBackend;
use nevermore_core::traits::{SessionBackend, TaskBackend, WindowRaiser};
use nevermore_core::SystemdTaskBackend;
use plugins::{create_default_registry, PluginRegistry};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub const PANEL_WIDTH: i32 = 400;
pub const NAV_RAIL_WIDTH: i32 = 60;
pub const CONTENT_WIDTH: i32 = 340;
pub const HOT_EDGE_WIDTH: i32 = 1;
pub const AUTO_HIDE_DELAY_MS: u32 = 250;
pub const PEEK_GRACE_DELAY_MS: u32 = 1500;

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
        // Lock GTK4 Settings to Built-In "Default" Theme on Startup
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_theme_name(Some("Default"));
            settings.set_gtk_application_prefer_dark_theme(true);
        }

        // Keep the application running resident in the background indefinitely
        std::mem::forget(app.hold());

        // 1. Initialize Semantic Theme Engine & Load Dynamic GTK4 CSS
        let _ = install_bundled_themes();
        let active_theme_name = get_active_theme_name();
        let mut theme_palette = load_theme(&active_theme_name).unwrap_or_default();

        let (saved_ui_font, saved_mono_font, saved_mono_size) = get_appearance_config();
        if let Some(ui_f) = saved_ui_font {
            theme_palette.ui_font = ui_f;
        }
        if let Some(mono_f) = saved_mono_font {
            theme_palette.mono_font = mono_f;
        }
        if let Some(mono_s) = saved_mono_size {
            theme_palette.mono_font_size = mono_s as u32;
        }

        let theme_cell = Rc::new(RefCell::new(theme_palette));

        let css_provider = CssProvider::new();
        css_provider.load_from_string(&theme_cell.borrow().to_gtk_css());
        let user_provider = CssProvider::new();

        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER + 100,
            );

            // Optional user override stylesheet at $XDG_CONFIG_HOME/nevermore/custom.css
            let custom_css = custom_css_path();
            if custom_css.exists() {
                if let Ok(custom_str) = std::fs::read_to_string(&custom_css) {
                    user_provider.load_from_string(&custom_str);
                    gtk4::style_context_add_provider_for_display(
                        &display,
                        &user_provider,
                        gtk4::STYLE_PROVIDER_PRIORITY_USER + 200,
                    );
                }
            }
        }

        // P2-1: Primary monitor binding
        let primary_monitor = gdk::Display::default().and_then(|disp| {
            let monitors = disp.monitors();
            if monitors.n_items() > 0 {
                monitors.item(0).and_downcast::<gdk::Monitor>()
            } else {
                None
            }
        });
        let monitor_height = primary_monitor
            .as_ref()
            .map(|m| m.geometry().height())
            .unwrap_or(1080);

        // 2A. P2-1: Build 1px Hot-Edge Layer-Shell Window with exclusive_zone = -1 and Layer::Overlay
        let hot_edge_window = ApplicationWindow::builder()
            .application(app)
            .title("Nevermore Hot Edge")
            .build();

        hot_edge_window.init_layer_shell();
        hot_edge_window.set_layer(Layer::Overlay);
        hot_edge_window.set_exclusive_zone(-1);
        hot_edge_window.set_anchor(Edge::Right, true);
        hot_edge_window.set_anchor(Edge::Top, true);
        hot_edge_window.set_anchor(Edge::Bottom, true);
        hot_edge_window.set_margin(Edge::Right, 0);
        hot_edge_window.set_margin(Edge::Top, 0);
        hot_edge_window.set_margin(Edge::Bottom, 0);
        hot_edge_window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
        hot_edge_window.set_decorated(false);
        hot_edge_window.add_css_class("hot-edge-window");

        if let Some(ref mon) = primary_monitor {
            hot_edge_window.set_monitor(Some(mon));
        }
        hot_edge_window.set_size_request(HOT_EDGE_WIDTH, monitor_height);
        hot_edge_window.set_default_size(HOT_EDGE_WIDTH, monitor_height);

        // Child Box with rgba(24, 24, 27, 0.01) background ensures full Wayland input region
        let hot_edge_box = GtkBox::new(Orientation::Vertical, 0);
        hot_edge_box.add_css_class("hot-edge-strip");
        hot_edge_box.set_vexpand(true);
        hot_edge_box.set_hexpand(true);
        hot_edge_box.set_size_request(HOT_EDGE_WIDTH, monitor_height);
        hot_edge_window.set_child(Some(&hot_edge_box));

        // 2B. P2-1: Build 400px Main Sidebar Panel Window with exclusive_zone = -1 and Layer::Overlay
        let panel_window = ApplicationWindow::builder()
            .application(app)
            .title("Nevermore Wayland Sidebar")
            .build();

        panel_window.init_layer_shell();
        panel_window.set_layer(Layer::Overlay);
        panel_window.set_exclusive_zone(-1);
        panel_window.set_anchor(Edge::Right, true);
        panel_window.set_anchor(Edge::Top, true);
        panel_window.set_anchor(Edge::Bottom, true);
        panel_window.set_margin(Edge::Right, 0);
        panel_window.set_margin(Edge::Top, 0);
        panel_window.set_margin(Edge::Bottom, 0);
        panel_window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
        panel_window.set_decorated(false);

        if let Some(ref mon) = primary_monitor {
            panel_window.set_monitor(Some(mon));
        }

        // Lock window width strictly to 400px, full monitor height
        panel_window.set_size_request(PANEL_WIDTH, monitor_height);
        panel_window.set_default_size(PANEL_WIDTH, monitor_height);
        panel_window.set_resizable(false);
        panel_window.add_css_class("sidebar-panel");

        // 3. State Machine & Domain Backends
        let state_machine = Arc::new(Mutex::new(SidebarStateMachine::new()));
        let tmux = TmuxBackend::new("nevermore");
        let systemd = SystemdTaskBackend::new("nevermore-task");
        let kwin = KWinAdapter::new();

        // 4. Modular Plugin Architecture Registry
        let registry = Rc::new(RefCell::new(create_default_registry(
            theme_cell.clone(),
            tmux.clone(),
            systemd.clone(),
            kwin.clone(),
        )));

        // P1-2: Synchronize dynamic registered plugins with state machine (including built-in settings)
        let mut valid_tabs = registry.borrow().plugin_ids();
        valid_tabs.push("__settings__".to_string());
        state_machine.lock().unwrap().set_valid_tabs(valid_tabs);

        // 5. Construct Layout locked to 400px (60px rail + 340px content), full monitor height
        let main_box = GtkBox::new(Orientation::Horizontal, 0);
        main_box.set_size_request(PANEL_WIDTH, monitor_height);
        main_box.set_hexpand(false);
        main_box.set_vexpand(true);

        // Content Area with Header
        let content_stack = Stack::new();
        content_stack.set_hexpand(true);
        content_stack.set_vexpand(true);

        registry.borrow().attach_to_stack(&content_stack);

        let nav_rail_cell: Rc<RefCell<Option<Rc<NavRail>>>> = Rc::new(RefCell::new(None));

        // Slim Icon-Only Header (~36px) preserving Zone Rule horizontal swipe
        let header = GtkBox::new(Orientation::Horizontal, 6);
        header.add_css_class("sidebar-header");

        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let btn_pin = Button::new();
        btn_pin.set_tooltip_text(Some("Toggle Pin"));
        btn_pin.update_property(&[Property::Label("Toggle Pin")]);
        btn_pin.add_css_class("btn-header-pin");
        icons::set_button_icon(&btn_pin, "pin", &theme_cell.borrow().text_secondary, 16);

        let btn_close = Button::new();
        btn_close.set_tooltip_text(Some("Close Sidebar (Escape)"));
        btn_close.update_property(&[Property::Label("Close Sidebar")]);
        btn_close.add_css_class("btn-header-close");
        icons::set_button_icon(&btn_close, "x", &theme_cell.borrow().text_secondary, 16);

        header.append(&spacer);
        header.append(&btn_pin);
        header.append(&btn_close);

        let btn_pin_theme = btn_pin.clone();
        let btn_close_theme = btn_close.clone();
        let sm_theme = state_machine.clone();
        let on_theme_updated = {
            let css_provider = css_provider.clone();
            let theme_cell = theme_cell.clone();
            let nav_rail = nav_rail_cell.clone();
            let registry = registry.clone();
            let btn_pin = btn_pin_theme;
            let btn_close = btn_close_theme;
            let sm = sm_theme;
            Rc::new(move |new_palette: &ThemePalette| {
                let (saved_ui_font, saved_mono_font, saved_mono_size) = get_appearance_config();
                let mut pal = new_palette.clone();
                if let Some(ui_f) = saved_ui_font {
                    pal.ui_font = ui_f;
                }
                if let Some(mono_f) = saved_mono_font {
                    pal.mono_font = mono_f;
                }
                if let Some(mono_s) = saved_mono_size {
                    pal.mono_font_size = mono_s as u32;
                }
                *theme_cell.borrow_mut() = pal.clone();
                let css = pal.to_gtk_css();
                css_provider.load_from_string(&css);
                let is_pinned = sm.lock().unwrap().state() == SidebarState::Pinned;
                if is_pinned {
                    btn_pin.add_css_class("pinned");
                    let accent_fg = contrast_text_color_for_hex(&pal.accent_color);
                    icons::set_button_icon(&btn_pin, "pin-off", accent_fg, 16);
                } else {
                    btn_pin.remove_css_class("pinned");
                    icons::set_button_icon(&btn_pin, "pin", &pal.text_secondary, 16);
                }
                icons::set_button_icon(&btn_close, "x", &pal.text_secondary, 16);
                btn_pin.queue_draw();
                btn_close.queue_draw();
                if let Some(ref nr) = *nav_rail.borrow() {
                    nr.on_theme_changed(&pal);
                }
                registry.borrow_mut().on_theme_changed(&pal);
            })
        };

        let btn_pin_app = btn_pin.clone();
        let btn_close_app = btn_close.clone();
        let sm_app = state_machine.clone();
        let on_appearance_updated = {
            let css_provider = css_provider.clone();
            let theme_cell = theme_cell.clone();
            let nav_rail = nav_rail_cell.clone();
            let registry = registry.clone();
            let btn_pin = btn_pin_app;
            let btn_close = btn_close_app;
            let sm = sm_app;
            Rc::new(move |ui_font: &str, mono_font: &str, mono_font_size: f64| {
                theme_cell.borrow_mut().ui_font = ui_font.to_string();
                theme_cell.borrow_mut().mono_font = mono_font.to_string();
                theme_cell.borrow_mut().mono_font_size = mono_font_size as u32;
                let cur = theme_cell.borrow().clone();
                let css = cur.to_gtk_css();
                css_provider.load_from_string(&css);
                let is_pinned = sm.lock().unwrap().state() == SidebarState::Pinned;
                if is_pinned {
                    btn_pin.add_css_class("pinned");
                    let accent_fg = contrast_text_color_for_hex(&cur.accent_color);
                    icons::set_button_icon(&btn_pin, "pin-off", accent_fg, 16);
                } else {
                    btn_pin.remove_css_class("pinned");
                    icons::set_button_icon(&btn_pin, "pin", &cur.text_secondary, 16);
                }
                icons::set_button_icon(&btn_close, "x", &cur.text_secondary, 16);
                btn_pin.queue_draw();
                btn_close.queue_draw();
                if let Some(ref nr) = *nav_rail.borrow() {
                    nr.on_theme_changed(&cur);
                }
                registry.borrow_mut().on_theme_changed(&cur);
            })
        };

        let settings_page_ctx = Rc::new(nevermore_ui::settings_page::SettingsPageContext {
            theme: theme_cell.clone(),
            nav_rail: nav_rail_cell.clone(),
            registry: registry.clone(),
            on_theme_updated,
            on_appearance_updated,
            tmux: tmux.clone(),
            systemd: systemd.clone(),
            kwin: kwin.clone(),
        });

        let settings_page_widget =
            nevermore_ui::settings_page::build_settings_page(settings_page_ctx);
        content_stack.add_named(&settings_page_widget, Some("__settings__"));

        let content_area = GtkBox::new(Orientation::Vertical, 0);
        content_area.set_size_request(CONTENT_WIDTH, monitor_height);
        content_area.set_hexpand(false);
        content_area.set_vexpand(true);

        content_area.append(&header);
        content_area.append(&content_stack);

        // Helper to detect open popovers / dropdown menus
        fn has_visible_popover(root: &impl IsA<gtk4::Widget>) -> bool {
            fn check_widget(w: &gtk4::Widget) -> bool {
                if w.is_visible() && (w.is::<gtk4::Popover>() || w.is::<gtk4::PopoverMenu>()) {
                    return true;
                }
                let mut child = w.first_child();
                while let Some(c) = child {
                    if check_widget(&c) {
                        return true;
                    }
                    child = c.next_sibling();
                }
                false
            }
            check_widget(root.as_ref())
        }

        // Shared Context for Visual Updates
        struct UiContext {
            hot_edge_window: ApplicationWindow,
            panel_window: ApplicationWindow,
            state_machine: Arc<Mutex<SidebarStateMachine>>,
            btn_pin: Button,
            btn_close: Button,
            nav_rail: Rc<RefCell<Option<Rc<NavRail>>>>,
            content_stack: Stack,
            leave_source: RefCell<Option<glib::SourceId>>,
            peek_grace_source: RefCell<Option<glib::SourceId>>,
            registry: Rc<RefCell<PluginRegistry>>,
            theme: Rc<RefCell<ThemePalette>>,
            is_pinned: Cell<bool>,
            panel_hovered: Cell<bool>,
        }

        let ctx = Rc::new(UiContext {
            hot_edge_window: hot_edge_window.clone(),
            panel_window: panel_window.clone(),
            state_machine: state_machine.clone(),
            btn_pin: btn_pin.clone(),
            btn_close: btn_close.clone(),
            nav_rail: nav_rail_cell.clone(),
            content_stack: content_stack.clone(),
            leave_source: RefCell::new(None),
            peek_grace_source: RefCell::new(None),
            registry: registry.clone(),
            theme: theme_cell.clone(),
            is_pinned: Cell::new(false),
            panel_hovered: Cell::new(false),
        });

        // Single authoritative apply_state synchronizer
        let apply_state = {
            let ctx = ctx.clone();
            Rc::new(move || {
                let sm = ctx.state_machine.lock().unwrap();
                let state = sm.state();
                let active_tab = sm.active_tab().to_string();
                let is_visible = state.is_visible();
                drop(sm);

                ctx.content_stack.set_visible_child_name(&active_tab);
                if let Some(ref nr) = *ctx.nav_rail.borrow() {
                    nr.set_active_tab(&active_tab);
                }

                let theme = ctx.theme.borrow();
                let currently_pinned = state == SidebarState::Pinned;
                if currently_pinned != ctx.is_pinned.get() {
                    ctx.is_pinned.set(currently_pinned);
                    if currently_pinned {
                        ctx.btn_pin.add_css_class("pinned");
                        let accent_fg = contrast_text_color_for_hex(&theme.accent_color);
                        icons::set_button_icon(&ctx.btn_pin, "pin-off", accent_fg, 16);
                    } else {
                        ctx.btn_pin.remove_css_class("pinned");
                        icons::set_button_icon(&ctx.btn_pin, "pin", &theme.text_secondary, 16);
                    }
                }

                match state {
                    SidebarState::Hidden => {
                        ctx.panel_window.set_visible(false);
                        ctx.hot_edge_window.set_visible(true);
                        ctx.hot_edge_window.present();
                        ctx.panel_window.remove_css_class("state-engaged");
                    }
                    SidebarState::Peek => {
                        ctx.hot_edge_window.set_visible(false);
                        ctx.panel_window.set_visible(true);
                        ctx.panel_window.present();
                        ctx.panel_window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
                        ctx.panel_window.remove_css_class("state-engaged");
                    }
                    SidebarState::Engaged => {
                        ctx.hot_edge_window.set_visible(false);
                        ctx.panel_window.set_visible(true);
                        ctx.panel_window.present();
                        ctx.panel_window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
                        ctx.panel_window.add_css_class("state-engaged");
                    }
                    SidebarState::Pinned => {
                        ctx.hot_edge_window.set_visible(false);
                        ctx.panel_window.set_visible(true);
                        ctx.panel_window.present();
                        ctx.panel_window
                            .set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
                        ctx.panel_window.remove_css_class("state-engaged");
                    }
                }
                drop(theme);

                // A4: Sync plugin lifecycle AFTER window mapping and keyboard mode configuration
                ctx.registry
                    .borrow_mut()
                    .sync_lifecycle(is_visible, &active_tab);
            })
        };

        // Construct NavRail routed through state machine
        let sm_for_nav = state_machine.clone();
        let apply_nav = apply_state.clone();
        let nav_rail = NavRail::new(
            &registry.borrow().manifests(),
            &theme_cell.borrow(),
            move |tab_name| {
                let mut sm = sm_for_nav.lock().unwrap();
                let _ = sm.set_active_tab(tab_name);
                drop(sm);
                apply_nav();
            },
        );
        nav_rail
            .widget()
            .set_size_request(NAV_RAIL_WIDTH, monitor_height);
        nav_rail.widget().set_hexpand(false);
        nav_rail.widget().set_vexpand(true);

        // Apply disabled plugins from config.toml on startup
        let disabled_plugins = get_disabled_plugins();
        for dis in &disabled_plugins {
            nav_rail.set_tab_visible(dis, false);
        }

        *ctx.nav_rail.borrow_mut() = Some(nav_rail.clone());

        // Attach horizontal swipe gesture controller to Header Bar (Fix 8 - Zone Rule)
        let header_scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::HORIZONTAL);
        let sm_for_header = state_machine.clone();
        let apply_header = apply_state.clone();
        header_scroll_ctrl.connect_scroll(move |_, dx, _| {
            if dx.abs() > 0.5 {
                let mut sm = sm_for_header.lock().unwrap();
                let cur = sm.active_tab().to_string();
                let tabs: Vec<String> = sm
                    .valid_tabs()
                    .iter()
                    .filter(|t| t.as_str() != "__settings__")
                    .cloned()
                    .collect();
                if tabs.is_empty() {
                    return glib::Propagation::Proceed;
                }
                let current_idx = tabs.iter().position(|id| *id == cur).unwrap_or(0);
                let next_idx = if dx > 0.0 {
                    (current_idx + 1) % tabs.len()
                } else if current_idx == 0 {
                    tabs.len() - 1
                } else {
                    current_idx - 1
                };
                let next_tab = &tabs[next_idx];
                let _ = sm.set_active_tab(next_tab);
                drop(sm);
                apply_header();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        header.add_controller(header_scroll_ctrl);

        main_box.append(nav_rail.widget());
        main_box.append(&content_area);
        panel_window.set_child(Some(&main_box));

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

        // Pointer Enter on Hot Edge Window (Hidden -> Peek + 1500ms Grace Timer)
        let edge_motion = EventControllerMotion::new();
        let apply_edge = apply_state.clone();
        let ctx_edge = ctx.clone();
        edge_motion.connect_enter(move |_, _, _| {
            if let Some(src) = ctx_edge.peek_grace_source.borrow_mut().take() {
                safe_source_remove(src);
            }

            ctx_edge
                .state_machine
                .lock()
                .unwrap()
                .transition(SidebarEvent::PointerEnterEdge);
            apply_edge();

            let ctx_grace = ctx_edge.clone();
            let apply_grace = apply_edge.clone();
            let source_id = glib::timeout_add_local(
                std::time::Duration::from_millis(PEEK_GRACE_DELAY_MS as u64),
                move || {
                    ctx_grace.peek_grace_source.borrow_mut().take();
                    let mut sm = ctx_grace.state_machine.lock().unwrap();
                    if sm.state() == SidebarState::Peek {
                        sm.transition(SidebarEvent::PointerLeaveEdge);
                        drop(sm);
                        apply_grace();
                    }
                    glib::ControlFlow::Break
                },
            );
            *ctx_edge.peek_grace_source.borrow_mut() = Some(source_id);
        });
        hot_edge_window.add_controller(edge_motion);

        let edge_box_motion = EventControllerMotion::new();
        let apply_edge_box = apply_state.clone();
        let ctx_edge_box = ctx.clone();
        edge_box_motion.connect_enter(move |_, _, _| {
            if let Some(src) = ctx_edge_box.peek_grace_source.borrow_mut().take() {
                safe_source_remove(src);
            }

            ctx_edge_box
                .state_machine
                .lock()
                .unwrap()
                .transition(SidebarEvent::PointerEnterEdge);
            apply_edge_box();

            let ctx_grace = ctx_edge_box.clone();
            let apply_grace = apply_edge_box.clone();
            let source_id = glib::timeout_add_local(
                std::time::Duration::from_millis(PEEK_GRACE_DELAY_MS as u64),
                move || {
                    ctx_grace.peek_grace_source.borrow_mut().take();
                    let mut sm = ctx_grace.state_machine.lock().unwrap();
                    if sm.state() == SidebarState::Peek {
                        sm.transition(SidebarEvent::PointerLeaveEdge);
                        drop(sm);
                        apply_grace();
                    }
                    glib::ControlFlow::Break
                },
            );
            *ctx_edge_box.peek_grace_source.borrow_mut() = Some(source_id);
        });
        hot_edge_box.add_controller(edge_box_motion);

        // Click inside Panel Window (Peek -> Engaged)
        let click_inside = GestureClick::new();
        let apply_click = apply_state.clone();
        let ctx_click = ctx.clone();
        click_inside.connect_pressed(move |_, _, _, _| {
            if let Some(src) = ctx_click.peek_grace_source.borrow_mut().take() {
                safe_source_remove(src);
            }
            let cur_state = ctx_click.state_machine.lock().unwrap().state();
            if cur_state == SidebarState::Peek || cur_state == SidebarState::Hidden {
                ctx_click
                    .state_machine
                    .lock()
                    .unwrap()
                    .transition(SidebarEvent::PointerClickInside);
                apply_click();
            }
        });
        panel_window.add_controller(click_inside);

        // Motion Controller on Panel Window (Cancel debounce on enter, auto-hide on leave)
        let panel_motion = EventControllerMotion::new();
        let ctx_leave = ctx.clone();

        panel_motion.connect_enter(move |_, _, _| {
            ctx_leave.panel_hovered.set(true);
            if let Some(src) = ctx_leave.peek_grace_source.borrow_mut().take() {
                safe_source_remove(src);
            }
            if let Some(source_id) = ctx_leave.leave_source.borrow_mut().take() {
                safe_source_remove(source_id);
            }
        });

        let ctx_timer_leave = ctx.clone();
        let apply_timer = apply_state.clone();
        panel_motion.connect_leave(move |_| {
            ctx_timer_leave.panel_hovered.set(false);
            if let Some(source_id) = ctx_timer_leave.leave_source.borrow_mut().take() {
                safe_source_remove(source_id);
            }

            let ctx_timer = ctx_timer_leave.clone();
            let apply_timer_cb = apply_timer.clone();

            let source_id = glib::timeout_add_local(
                std::time::Duration::from_millis(AUTO_HIDE_DELAY_MS as u64),
                move || {
                    if ctx_timer.panel_hovered.get() {
                        ctx_timer.leave_source.borrow_mut().take();
                        return glib::ControlFlow::Break;
                    }
                    if has_visible_popover(&ctx_timer.panel_window) {
                        return glib::ControlFlow::Continue;
                    }
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
                if let Some(src) = ctx_key.peek_grace_source.borrow_mut().take() {
                    safe_source_remove(src);
                }
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
        let theme_ipc = theme_cell.clone();
        let css_prov_ipc = css_provider.clone();
        let user_prov_ipc = user_provider.clone();
        let reg_ipc = registry.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = ipc_rx.recv().await {
                let start_t = Instant::now();
                match cmd.request {
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
                        let _ = cmd.responder.send(IpcResponse::State {
                            state: next,
                            active_tab: tab,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        });
                    }
                    nevermore_core::ipc::IpcRequest::Show { tab } => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        if let Some(ref t) = tab {
                            if !sm.is_valid_tab(t) {
                                let valid = sm.valid_tabs().to_vec();
                                drop(sm);
                                let _ = cmd.responder.send(IpcResponse::Error(format!(
                                    "Invalid tab '{}'. Valid tabs are: {:?}",
                                    t, valid
                                )));
                            } else {
                                let next = sm.transition(SidebarEvent::CliShow { tab });
                                let active = sm.active_tab().to_string();
                                drop(sm);
                                apply_ipc();
                                let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                                println!(
                                    "[IPC] Show executed in {:.2} ms -> {:?}",
                                    latency_ms, next
                                );
                                let _ = cmd.responder.send(IpcResponse::State {
                                    state: next,
                                    active_tab: active,
                                    is_visible: next.is_visible(),
                                    takes_focus: next.takes_keyboard_focus(),
                                });
                            }
                        } else {
                            let next = sm.transition(SidebarEvent::CliShow { tab });
                            let active = sm.active_tab().to_string();
                            drop(sm);
                            apply_ipc();
                            let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                            println!("[IPC] Show executed in {:.2} ms -> {:?}", latency_ms, next);
                            let _ = cmd.responder.send(IpcResponse::State {
                                state: next,
                                active_tab: active,
                                is_visible: next.is_visible(),
                                takes_focus: next.takes_keyboard_focus(),
                            });
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
                        let _ = cmd.responder.send(IpcResponse::State {
                            state: next,
                            active_tab: tab,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        });
                    }
                    nevermore_core::ipc::IpcRequest::Pin => {
                        let mut sm = ctx_ipc.state_machine.lock().unwrap();
                        let next = sm.transition(SidebarEvent::CliPin);
                        let tab = sm.active_tab().to_string();
                        drop(sm);
                        apply_ipc();
                        let latency_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                        println!("[IPC] Pin executed in {:.2} ms -> {:?}", latency_ms, next);
                        let _ = cmd.responder.send(IpcResponse::State {
                            state: next,
                            active_tab: tab,
                            is_visible: next.is_visible(),
                            takes_focus: next.takes_keyboard_focus(),
                        });
                    }
                    nevermore_core::ipc::IpcRequest::Status => {
                        let sm = ctx_ipc.state_machine.lock().unwrap();
                        let _ = cmd.responder.send(IpcResponse::State {
                            state: sm.state(),
                            active_tab: sm.active_tab().to_string(),
                            is_visible: sm.state().is_visible(),
                            takes_focus: sm.state().takes_keyboard_focus(),
                        });
                    }
                    // A2: Offload blocking IPC arms to gio::spawn_blocking
                    nevermore_core::ipc::IpcRequest::ListSessions => {
                        let tmux = tmux_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || match tmux.list_sessions() {
                                Ok(sessions) => IpcResponse::Sessions(sessions),
                                Err(e) => IpcResponse::Error(e.to_string()),
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::CreateSession { name } => {
                        let tmux = tmux_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res =
                                gio::spawn_blocking(move || match tmux.create_session(&name) {
                                    Ok(_) => IpcResponse::Ok,
                                    Err(e) => IpcResponse::Error(e.to_string()),
                                })
                                .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::KillSession { id } => {
                        let tmux = tmux_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || match tmux.kill_session(&id) {
                                Ok(_) => IpcResponse::Ok,
                                Err(e) => IpcResponse::Error(e.to_string()),
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::ListTasks => {
                        let systemd = systemd_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || match systemd.list_tasks() {
                                Ok(tasks) => IpcResponse::Tasks(tasks),
                                Err(e) => IpcResponse::Error(e.to_string()),
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::LaunchTask {
                        command,
                        working_dir,
                    } => {
                        let systemd = systemd_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || {
                                match systemd.launch_task(&command, &working_dir) {
                                    Ok(_) => IpcResponse::Ok,
                                    Err(e) => IpcResponse::Error(e.to_string()),
                                }
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::GetTaskLogs { unit, line_count } => {
                        let systemd = systemd_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || {
                                match systemd.get_task_logs(&unit, line_count) {
                                    Ok(logs) => IpcResponse::Logs(logs),
                                    Err(e) => IpcResponse::Error(e.to_string()),
                                }
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::StopTask { unit } => {
                        let systemd = systemd_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || match systemd.stop_task(&unit) {
                                Ok(_) => IpcResponse::Ok,
                                Err(e) => IpcResponse::Error(e.to_string()),
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::RaiseApp { desktop_entry } => {
                        let kwin = kwin_ipc.clone();
                        let responder = cmd.responder;
                        glib::spawn_future_local(async move {
                            let res = gio::spawn_blocking(move || {
                                match kwin.raise_application(&desktop_entry) {
                                    Ok(_) => IpcResponse::Ok,
                                    Err(e) => IpcResponse::Error(e.to_string()),
                                }
                            })
                            .await;
                            let response = res.unwrap_or_else(|_| {
                                IpcResponse::Error("Worker thread panicked".to_string())
                            });
                            let _ = responder.send(response);
                        });
                    }
                    nevermore_core::ipc::IpcRequest::ListThemes => {
                        let active = get_active_theme_name();
                        let available = list_available_themes();
                        let _ = cmd
                            .responder
                            .send(IpcResponse::Themes { active, available });
                    }
                    nevermore_core::ipc::IpcRequest::SetTheme { name } => {
                        let response = match load_theme(&name) {
                            Ok(mut new_palette) => {
                                let (ui_f, mono_f, mono_sz) = get_appearance_config();
                                if let Some(f) = ui_f {
                                    new_palette.ui_font = f;
                                }
                                if let Some(mf) = mono_f {
                                    new_palette.mono_font = mf;
                                }
                                if let Some(msz) = mono_sz {
                                    new_palette.mono_font_size = msz as u32;
                                }
                                *theme_ipc.borrow_mut() = new_palette.clone();
                                let _ = set_active_theme_name(&name);
                                css_prov_ipc.load_from_string(&new_palette.to_gtk_css());

                                // Reload custom.css if present
                                let custom_css = custom_css_path();
                                if custom_css.exists() {
                                    if let Ok(custom_str) = std::fs::read_to_string(&custom_css) {
                                        user_prov_ipc.load_from_string(&custom_str);
                                    }
                                }

                                reg_ipc.borrow_mut().on_theme_changed(&new_palette);
                                if let Some(ref nr) = *ctx_ipc.nav_rail.borrow() {
                                    nr.on_theme_changed(&new_palette);
                                }
                                icons::set_button_icon(
                                    &ctx_ipc.btn_close,
                                    "x",
                                    &new_palette.text_secondary,
                                    16,
                                );
                                apply_ipc();
                                println!("[THEME] Switched active theme live to '{}'", name);
                                IpcResponse::Ok
                            }
                            Err(e) => IpcResponse::Error(e),
                        };
                        let _ = cmd.responder.send(response);
                    }
                }
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
        let hw_log = hot_edge_window.clone();
        let hb_log = hot_edge_box.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            println!(
                "[HOT_EDGE] Allocated geometry: window width={}, window height={}, box height={}",
                hw_log.width(),
                hw_log.height(),
                hb_log.height()
            );
        });
        apply_state();
        hot_edge_window.present();

        println!(
            "[NEVERMORE] UI initialized. Initial RSS: {:.2} MB",
            get_process_rss_mb()
        );
    });

    app.run();
}
