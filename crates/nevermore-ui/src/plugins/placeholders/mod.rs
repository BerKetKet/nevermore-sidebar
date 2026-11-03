use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Widget};
use nevermore_core::plugin::PluginManifest;

pub struct PlaceholderPlugin {
    manifest: PluginManifest,
}

impl PlaceholderPlugin {
    pub fn new(id: &str, title: &str, icon: &str, order: u32, desc: &str) -> Self {
        Self {
            manifest: PluginManifest {
                id: id.to_string(),
                title: title.to_string(),
                icon: icon.to_string(),
                order,
                description: desc.to_string(),
            },
        }
    }
}

impl SidebarPlugin for PlaceholderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn build_widget(&mut self, _ctx: &PluginContext) -> Widget {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_valign(gtk4::Align::Center);
        container.set_halign(gtk4::Align::Center);

        let title_text = if self.manifest.id == "email" {
            "NEVERMORE // EMAIL\nEmail Feasibility — Phase 11 Spike".to_string()
        } else {
            format!(
                "NEVERMORE // {}\n(Phase 8-10 Tab)",
                self.manifest.title.to_uppercase()
            )
        };

        let label = Label::new(Some(&title_text));
        label.add_css_class("header-title");
        label.set_justify(gtk4::Justification::Center);
        container.append(&label);

        container.upcast::<Widget>()
    }
}
