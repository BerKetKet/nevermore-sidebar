pub mod pane;

use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::Widget;
use nevermore_core::plugin::PluginManifest;
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
        let pane = NotificationsPane::new(ctx.kwin.clone());
        let widget = pane.widget().clone().upcast::<Widget>();
        self.pane = Some(pane);
        widget
    }
}
