pub mod pane;

use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::Widget;
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
pub use pane::NotepadPane;
use std::rc::Rc;

pub struct NotepadPlugin {
    manifest: PluginManifest,
    pane: Option<Rc<NotepadPane>>,
}

impl NotepadPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest::from_toml(include_str!("plugin.toml"))
            .expect("valid notepad plugin.toml");
        Self {
            manifest,
            pane: None,
        }
    }
}

impl Default for NotepadPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarPlugin for NotepadPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn build_widget(&mut self, ctx: &PluginContext) -> Widget {
        let notes_file = ctx.data_dir.join("notes.txt");
        let theme = ctx.theme();
        let pane = NotepadPane::new(notes_file, &theme);
        let widget = pane.widget().clone().upcast::<Widget>();
        self.pane = Some(pane);
        widget
    }

    fn on_hide(&mut self) {
        if let Some(ref pane) = self.pane {
            pane.flush_save();
        }
    }

    fn on_theme_changed(&mut self, theme: &ThemePalette) {
        if let Some(ref pane) = self.pane {
            pane.apply_theme(theme);
        }
    }
}
