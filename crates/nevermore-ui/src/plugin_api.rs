use gtk4::prelude::*;
use gtk4::{Overflow, PolicyType, ScrolledWindow, Widget};
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
use nevermore_core::tmux::TmuxBackend;
use nevermore_core::SystemdTaskBackend;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

pub const PLUGIN_CONTENT_WIDTH: i32 = 340;

/// PluginContext: Safe capability struct passed to each plugin.
#[derive(Clone)]
#[allow(dead_code)]
pub struct PluginContext {
    pub plugin_id: String,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    theme: Rc<RefCell<ThemePalette>>,
    pub tmux: TmuxBackend,
    pub systemd: SystemdTaskBackend,
    pub kwin: KWinAdapter,
}

impl PluginContext {
    pub fn new(
        plugin_id: &str,
        theme: Rc<RefCell<ThemePalette>>,
        tmux: TmuxBackend,
        systemd: SystemdTaskBackend,
        kwin: KWinAdapter,
    ) -> Self {
        let data_dir = if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
            PathBuf::from(xdg)
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else if let Some(home) = env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else {
            PathBuf::from(format!("/tmp/nevermore-data/plugins/{}", plugin_id))
        };

        let config_dir = if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else if let Some(home) = env::var_os("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else {
            PathBuf::from(format!("/tmp/nevermore-config/plugins/{}", plugin_id))
        };

        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&config_dir);

        Self {
            plugin_id: plugin_id.to_string(),
            data_dir,
            config_dir,
            theme,
            tmux,
            systemd,
            kwin,
        }
    }

    /// Read-only getter for the current theme palette.
    pub fn theme(&self) -> ThemePalette {
        self.theme.borrow().clone()
    }
}

pub trait SidebarPlugin {
    fn manifest(&self) -> &PluginManifest;
    fn build_widget(&mut self, ctx: &PluginContext) -> Widget;
    fn manages_own_scroll(&self) -> bool {
        false
    }
    fn on_show(&mut self) {}
    fn on_hide(&mut self) {}
    fn on_theme_changed(&mut self, _theme: &ThemePalette) {}
}

/// Helper to wrap every plugin widget in a true 340px-clamped ScrolledWindow container.
pub fn wrap_in_plugin_container(widget: &Widget, manages_own_scroll: bool) -> ScrolledWindow {
    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(if manages_own_scroll {
            PolicyType::Never
        } else {
            PolicyType::Automatic
        })
        .propagate_natural_width(false)
        .min_content_width(PLUGIN_CONTENT_WIDTH)
        .max_content_width(PLUGIN_CONTENT_WIDTH)
        .hexpand(false)
        .vexpand(true)
        .build();
    scrolled.set_size_request(PLUGIN_CONTENT_WIDTH, -1);
    scrolled.set_overflow(Overflow::Hidden);
    scrolled.set_child(Some(widget));
    scrolled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_in_plugin_container_clamp() {
        if gtk4::init().is_err() {
            eprintln!("Skipping container clamp test: no display available");
            return;
        }
        let box_widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        box_widget.set_size_request(1000, 50);
        let container = wrap_in_plugin_container(&box_widget.upcast::<Widget>(), false);
        assert_eq!(container.min_content_width(), PLUGIN_CONTENT_WIDTH);
        assert_eq!(container.max_content_width(), PLUGIN_CONTENT_WIDTH);
        assert!(!container.propagates_natural_width());
    }
}
