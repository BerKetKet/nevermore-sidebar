pub mod pane;
pub mod parser;

use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::Widget;
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
pub use pane::NotificationsPane;
use std::rc::Rc;

pub struct NotificationsPlugin {
    manifest: PluginManifest,
    pane: Option<Rc<NotificationsPane>>,
}

impl NotificationsPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest::from_toml(include_str!("plugin.toml"))
            .expect("valid notifications plugin.toml");
        Self {
            manifest,
            pane: None,
        }
    }
}

impl Default for NotificationsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarPlugin for NotificationsPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn build_widget(&mut self, ctx: &PluginContext) -> Widget {
        let pane = NotificationsPane::new(ctx.kwin.clone(), &ctx.theme());
        let widget = pane.widget().clone().upcast::<Widget>();
        self.pane = Some(pane);
        widget
    }

    fn on_show(&mut self) {
        if let Some(ref pane) = self.pane {
            pane.update_relative_times();
            pane.refresh_dnd_state();
        }
    }

    fn on_theme_changed(&mut self, theme: &ThemePalette) {
        if let Some(ref pane) = self.pane {
            pane.apply_theme(theme);
        }
    }
}
