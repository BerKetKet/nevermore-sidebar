use crate::icons;
use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Widget};
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

    fn build_widget(&mut self, ctx: &PluginContext) -> Widget {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_valign(gtk4::Align::Center);
        container.set_halign(gtk4::Align::Center);

        let icon_name = if !self.manifest.icon.is_empty() {
            self.manifest.icon.as_str()
        } else {
            self.manifest.id.as_str()
        };

        let theme = ctx.theme();
        let icon_img = icons::create_icon_widget(icon_name, &theme.text_secondary, 40);
        icon_img.set_opacity(0.6);
        container.append(&icon_img);

        container.upcast::<Widget>()
    }
}
