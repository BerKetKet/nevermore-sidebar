pub mod notepad;
pub mod notifications;
pub mod placeholders;
pub mod terminal;
pub mod widgets;

use crate::plugin_api::{wrap_in_plugin_container, PluginContext, SidebarPlugin};
use gtk4::{ScrolledWindow, Stack};
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PluginEntry {
    pub manifest: PluginManifest,
    pub plugin: Box<dyn SidebarPlugin>,
    pub container: ScrolledWindow,
}

pub struct PluginRegistry {
    entries: Vec<PluginEntry>,
    shown_tab: Option<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            shown_tab: None,
        }
    }

    #[allow(dead_code)]
    pub fn shown_tab(&self) -> Option<&str> {
        self.shown_tab.as_deref()
    }

    pub fn plugin_ids(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.manifest.id.clone()).collect()
    }

    pub fn register(&mut self, mut plugin: Box<dyn SidebarPlugin>, ctx: &PluginContext) {
        let manifest = plugin.manifest().clone();
        if self.entries.iter().any(|e| e.manifest.id == manifest.id) {
            eprintln!(
                "[nevermore-plugin] warning: plugin ID '{}' already registered, skipping duplicate",
                manifest.id
            );
            return;
        }
        let manages_scroll = plugin.manages_own_scroll();
        let widget = plugin.build_widget(ctx);
        let container = wrap_in_plugin_container(&widget, manages_scroll);
        self.entries.push(PluginEntry {
            manifest,
            plugin,
            container,
        });
    }

    #[allow(dead_code)]
    pub fn register_for_test(&mut self, plugin: Box<dyn SidebarPlugin>) {
        let manifest = plugin.manifest().clone();
        if self.entries.iter().any(|e| e.manifest.id == manifest.id) {
            return;
        }
        let container = ScrolledWindow::new();
        self.entries.push(PluginEntry {
            manifest,
            plugin,
            container,
        });
    }

    pub fn sort_by_order(&mut self) {
        self.entries.sort_by_key(|e| e.manifest.order);
    }

    pub fn manifests(&self) -> Vec<PluginManifest> {
        self.entries.iter().map(|e| e.manifest.clone()).collect()
    }

    pub fn attach_to_stack(&self, stack: &Stack) {
        for entry in &self.entries {
            stack.add_named(&entry.container, Some(&entry.manifest.id));
        }
    }

    /// Single authoritative, idempotent lifecycle synchronizer (P0-1).
    pub fn sync_lifecycle(&mut self, is_visible: bool, active_tab: &str) {
        let target: Option<&str> = if is_visible { Some(active_tab) } else { None };

        // Idempotent guard: if already in the target state, early return (L3 fixed)
        if self.shown_tab.as_deref() == target {
            return;
        }

        // Hide previous active plugin if one was shown
        if let Some(old_id) = self.shown_tab.take() {
            if let Some(entry) = self.entries.iter_mut().find(|e| e.manifest.id == old_id) {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    entry.plugin.on_hide();
                }));
            }
        }

        // Show new active plugin if panel is visible
        if let Some(new_id) = target {
            if let Some(entry) = self.entries.iter_mut().find(|e| e.manifest.id == new_id) {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    entry.plugin.on_show();
                }));
                self.shown_tab = Some(new_id.to_string());
            }
        }
    }

    pub fn on_theme_changed(&mut self, theme: &ThemePalette) {
        for entry in &mut self.entries {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                entry.plugin.on_theme_changed(theme);
            }));
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_default_registry(
    theme: Rc<RefCell<ThemePalette>>,
    tmux: nevermore_core::TmuxBackend,
    systemd: nevermore_core::SystemdTaskBackend,
    kwin: nevermore_core::KWinAdapter,
) -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    // 1. Terminal
    let ctx_term = PluginContext::new(
        "terminal",
        theme.clone(),
        tmux.clone(),
        systemd.clone(),
        kwin.clone(),
    );
    registry.register(Box::new(terminal::TerminalPlugin::new()), &ctx_term);

    // 2. Widgets
    let ctx_widg = PluginContext::new(
        "widgets",
        theme.clone(),
        tmux.clone(),
        systemd.clone(),
        kwin.clone(),
    );
    registry.register(Box::new(widgets::WidgetsPlugin::new()), &ctx_widg);

    // 3. Notifications
    let ctx_notif = PluginContext::new(
        "notifications",
        theme.clone(),
        tmux.clone(),
        systemd.clone(),
        kwin.clone(),
    );
    registry.register(
        Box::new(notifications::NotificationsPlugin::new()),
        &ctx_notif,
    );

    // 4. Notepad
    let ctx_note = PluginContext::new(
        "notepad",
        theme.clone(),
        tmux.clone(),
        systemd.clone(),
        kwin.clone(),
    );
    registry.register(Box::new(notepad::NotepadPlugin::new()), &ctx_note);

    // 5-9 Placeholders
    let placeholders = [
        (
            "media",
            "Media Player",
            "media.svg",
            5,
            "Media player playback and controls",
        ),
        (
            "gemini",
            "Gemini Chat",
            "gemini.svg",
            6,
            "Gemini AI assistant inline chat",
        ),
        (
            "youtube",
            "YouTube",
            "youtube.svg",
            7,
            "YouTube search and queue",
        ),
        (
            "vpn",
            "VPN Status",
            "vpn.svg",
            8,
            "VPN wireguard status and toggle",
        ),
        ("email", "Email", "email.svg", 9, "Email inbox overview"),
    ];

    for (id, title, icon, order, desc) in placeholders {
        let ctx_p = PluginContext::new(
            id,
            theme.clone(),
            tmux.clone(),
            systemd.clone(),
            kwin.clone(),
        );
        registry.register(
            Box::new(placeholders::PlaceholderPlugin::new(
                id, title, icon, order, desc,
            )),
            &ctx_p,
        );
    }

    registry.sort_by_order();
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::Widget;

    struct LifecycleMockPlugin {
        manifest: PluginManifest,
        show_count: Rc<RefCell<usize>>,
        hide_count: Rc<RefCell<usize>>,
    }

    impl LifecycleMockPlugin {
        fn new(id: &str, show: Rc<RefCell<usize>>, hide: Rc<RefCell<usize>>) -> Self {
            Self {
                manifest: PluginManifest {
                    id: id.to_string(),
                    title: id.to_uppercase(),
                    icon: "test.svg".to_string(),
                    order: 1,
                    description: "mock".to_string(),
                },
                show_count: show,
                hide_count: hide,
            }
        }
    }

    impl SidebarPlugin for LifecycleMockPlugin {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn build_widget(&mut self, _ctx: &PluginContext) -> Widget {
            unreachable!("not needed for headless lifecycle testing")
        }

        fn on_show(&mut self) {
            *self.show_count.borrow_mut() += 1;
        }

        fn on_hide(&mut self) {
            *self.hide_count.borrow_mut() += 1;
        }
    }

    #[test]
    fn test_sync_lifecycle_state_machine() {
        let _ = gtk4::init();

        let w_show = Rc::new(RefCell::new(0));
        let w_hide = Rc::new(RefCell::new(0));
        let t_show = Rc::new(RefCell::new(0));
        let t_hide = Rc::new(RefCell::new(0));

        let mut reg = PluginRegistry::new();
        reg.register_for_test(Box::new(LifecycleMockPlugin::new(
            "widgets",
            w_show.clone(),
            w_hide.clone(),
        )));
        reg.register_for_test(Box::new(LifecycleMockPlugin::new(
            "terminal",
            t_show.clone(),
            t_hide.clone(),
        )));

        // 1. Initially hidden: shown_tab is None
        assert_eq!(reg.shown_tab(), None);

        // 2. Hidden -> Show("widgets") calls widgets.on_show() once
        reg.sync_lifecycle(true, "widgets");
        assert_eq!(*w_show.borrow(), 1);
        assert_eq!(*w_hide.borrow(), 0);
        assert_eq!(reg.shown_tab(), Some("widgets"));

        // 3. Re-clicking "widgets" while visible is a no-op (on_hide is NOT called!)
        reg.sync_lifecycle(true, "widgets");
        assert_eq!(*w_show.borrow(), 1);
        assert_eq!(*w_hide.borrow(), 0);
        assert_eq!(reg.shown_tab(), Some("widgets"));

        // 4. Show("widgets") -> Show("terminal") calls widgets.on_hide() and terminal.on_show()
        reg.sync_lifecycle(true, "terminal");
        assert_eq!(*w_hide.borrow(), 1);
        assert_eq!(*t_show.borrow(), 1);
        assert_eq!(*t_hide.borrow(), 0);
        assert_eq!(reg.shown_tab(), Some("terminal"));

        // 5. Hide: terminal.on_hide() runs, leaving shown_tab == None
        reg.sync_lifecycle(false, "terminal");
        assert_eq!(*t_hide.borrow(), 1);
        assert_eq!(reg.shown_tab(), None);
    }
}
