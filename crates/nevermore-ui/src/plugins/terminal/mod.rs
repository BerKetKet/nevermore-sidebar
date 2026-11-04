pub mod pane;

use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::Widget;
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
pub use pane::TerminalPane;
use std::rc::Rc;

pub struct TerminalPlugin {
    manifest: PluginManifest,
    pane: Option<Rc<TerminalPane>>,
}

impl TerminalPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest::from_toml(include_str!("plugin.toml"))
            .expect("valid terminal plugin.toml");
        Self {
            manifest,
            pane: None,
        }
    }
}

impl Default for TerminalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarPlugin for TerminalPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn manages_own_scroll(&self) -> bool {
        true
    }

    fn build_widget(&mut self, ctx: &PluginContext) -> Widget {
        let pane = TerminalPane::new(ctx.tmux.clone(), ctx.systemd.clone(), &ctx.theme());
        let widget = pane.widget().clone().upcast::<Widget>();
        self.pane = Some(pane);
        widget
    }

    fn on_show(&mut self) {
        if let Some(ref pane) = self.pane {
            pane.grab_terminal_focus();
        }
    }

    fn on_theme_changed(&mut self, theme: &ThemePalette) {
        if let Some(ref pane) = self.pane {
            pane.apply_theme(theme);
        }
    }
}
