pub mod pane;

use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::Widget;
use nevermore_core::plugin::PluginManifest;
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

    fn build_widget(&mut self, _ctx: &PluginContext) -> Widget {
        let pane = WidgetsPane::new();
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
}
