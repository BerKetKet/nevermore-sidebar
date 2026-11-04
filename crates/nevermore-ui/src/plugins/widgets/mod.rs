pub mod pane;

use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::Widget;
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
pub use pane::WidgetsPane;
use std::rc::Rc;

pub struct WidgetsPlugin {
    manifest: PluginManifest,
    pane: Option<Rc<WidgetsPane>>,
}

impl WidgetsPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest::from_toml(include_str!("plugin.toml"))
            .expect("valid widgets plugin.toml");
        Self {
            manifest,
            pane: None,
        }
    }
}

impl Default for WidgetsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarPlugin for WidgetsPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn build_widget(&mut self, ctx: &PluginContext) -> Widget {
        let theme = ctx.theme();
        let pane = WidgetsPane::new(&theme);
        let widget = pane.widget().clone().upcast::<Widget>();
        self.pane = Some(pane);
        widget
    }

    fn on_show(&mut self) {
        if let Some(ref pane) = self.pane {
            pane.on_show();
        }
    }

    fn on_hide(&mut self) {
        if let Some(ref pane) = self.pane {
            pane.on_hide();
        }
    }

    fn on_theme_changed(&mut self, theme: &ThemePalette) {
        if let Some(ref pane) = self.pane {
            pane.apply_theme(theme);
        }
    }
}
