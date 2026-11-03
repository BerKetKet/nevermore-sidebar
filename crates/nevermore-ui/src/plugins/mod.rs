pub mod notepad;
pub mod notifications;
pub mod placeholders;
pub mod terminal;
pub mod widgets;

use crate::plugin_api::{wrap_in_plugin_container, PluginContext, SidebarPlugin};
use gtk4::{Box as GtkBox, Stack};
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PluginEntry {
    pub manifest: PluginManifest,
    pub plugin: Box<dyn SidebarPlugin>,
    pub container: GtkBox,
}

pub struct PluginRegistry {
    entries: Vec<PluginEntry>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register(&mut self, mut plugin: Box<dyn SidebarPlugin>, ctx: &PluginContext) {
        let manifest = plugin.manifest().clone();
        let widget = plugin.build_widget(ctx);
        let container = wrap_in_plugin_container(&widget);
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

    pub fn on_show(&mut self, active_tab: &str) {
        for entry in &mut self.entries {
            if entry.manifest.id == active_tab {
                entry.plugin.on_show();
            }
        }
    }

    pub fn on_hide(&mut self, active_tab: &str) {
        for entry in &mut self.entries {
            if entry.manifest.id == active_tab {
                entry.plugin.on_hide();
            }
        }
    }

    pub fn on_tab_changed(&mut self, prev_tab: &str, new_tab: &str) {
        for entry in &mut self.entries {
            if entry.manifest.id == prev_tab {
                entry.plugin.on_hide();
            } else if entry.manifest.id == new_tab {
                entry.plugin.on_show();
            }
        }
    }

    pub fn on_theme_changed(&mut self, theme: &ThemePalette) {
        for entry in &mut self.entries {
            entry.plugin.on_theme_changed(theme);
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
