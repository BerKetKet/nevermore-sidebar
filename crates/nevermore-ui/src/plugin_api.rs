use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Overflow, Widget};
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
    pub theme: Rc<RefCell<ThemePalette>>,
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
}

pub trait SidebarPlugin {
    fn manifest(&self) -> &PluginManifest;
    fn build_widget(&mut self, ctx: &PluginContext) -> Widget;
    fn on_show(&mut self) {}
    fn on_hide(&mut self) {}
    fn on_theme_changed(&mut self, _theme: &ThemePalette) {}
}

/// Helper to wrap any plugin widget in a 340px-clamped shell container with Overflow::Hidden.
pub fn wrap_in_plugin_container(widget: &Widget) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.set_size_request(PLUGIN_CONTENT_WIDTH, -1);
    container.set_hexpand(false);
    container.set_vexpand(true);
    container.set_overflow(Overflow::Hidden);
    container.append(widget);
    container
}
