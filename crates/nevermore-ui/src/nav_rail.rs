use crate::icons;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    accessible::Property, Box as GtkBox, Button, EventControllerScroll, EventControllerScrollFlags,
    Orientation,
};
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
use std::cell::RefCell;
use std::rc::Rc;

pub struct NavRail {
    container: GtkBox,
    buttons: Vec<(String, Button)>,
    manifests: Vec<PluginManifest>,
    active_tab: Rc<RefCell<String>>,
    tabs_order: Vec<String>,
    palette: Rc<RefCell<ThemePalette>>,
}

impl NavRail {
    pub fn new<F>(
        manifests: &[PluginManifest],
        initial_palette: &ThemePalette,
        on_tab_switched: F,
    ) -> Rc<Self>
    where
        F: Fn(&str) + 'static + Clone,
    {
        let container = GtkBox::new(Orientation::Vertical, 4);
        container.set_size_request(60, -1);
        container.set_hexpand(false);
        container.add_css_class("nav-rail");

        let active_tab = Rc::new(RefCell::new("terminal".to_string()));
        let palette = Rc::new(RefCell::new(initial_palette.clone()));
        let mut buttons = Vec::new();
        let mut tabs_order = Vec::new();
        let stored_manifests = manifests.to_vec();

        for manifest in manifests {
            let id = &manifest.id;
            tabs_order.push(id.clone());

            let btn = Button::new();
            btn.add_css_class("nav-btn");
            btn.set_tooltip_text(Some(&manifest.title));
            btn.update_property(&[Property::Label(&manifest.title)]);

            let icon_name = if !manifest.icon.is_empty() {
                manifest.icon.as_str()
            } else {
                manifest.id.as_str()
            };

            let is_active = id == "terminal";
            if is_active {
                btn.add_css_class("active");
                icons::set_button_icon(&btn, icon_name, &initial_palette.accent_color, 20);
            } else {
                icons::set_button_icon(&btn, icon_name, &initial_palette.text_secondary, 20);
            }

            let tab_id = id.clone();
            let on_switch = on_tab_switched.clone();

            btn.connect_clicked(move |_| {
                on_switch(&tab_id);
            });

            container.append(&btn);
            buttons.push((id.clone(), btn));
        }

        // Expanding vertical spacer separating plugin tabs from bottom-pinned settings button
        let spacer = GtkBox::new(Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        container.append(&spacer);

        // Bottom-pinned settings gear icon button
        let settings_btn = Button::new();
        settings_btn.add_css_class("nav-btn");
        settings_btn.set_tooltip_text(Some("Settings"));
        settings_btn.update_property(&[Property::Label("Settings")]);
        icons::set_button_icon(
            &settings_btn,
            "settings",
            &initial_palette.text_secondary,
            20,
        );

        let on_switch_settings = on_tab_switched.clone();
        settings_btn.connect_clicked(move |_| {
            on_switch_settings("__settings__");
        });
        container.append(&settings_btn);
        buttons.push(("__settings__".to_string(), settings_btn));

        let nav = Rc::new(Self {
            container,
            buttons,
            manifests: stored_manifests,
            active_tab,
            tabs_order,
            palette,
        });

        // Horizontal swipe gesture on Nav Rail (Fix 8 - Zone Rule)
        let scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::HORIZONTAL);
        let nav_weak = Rc::downgrade(&nav);
        let on_switch = on_tab_switched;
        scroll_ctrl.connect_scroll(move |_, dx, _| {
            if let Some(n) = nav_weak.upgrade() {
                if dx.abs() > 0.5 {
                    let visible_tabs: Vec<&String> = n
                        .tabs_order
                        .iter()
                        .filter(|id| {
                            n.buttons
                                .iter()
                                .find(|(bid, _)| bid == *id)
                                .map(|(_, btn)| btn.is_visible())
                                .unwrap_or(false)
                        })
                        .collect();

                    if visible_tabs.is_empty() {
                        return glib::Propagation::Proceed;
                    }

                    let cur = n.active_tab.borrow().clone();
                    let current_idx = visible_tabs.iter().position(|id| **id == cur).unwrap_or(0);
                    let next_idx = if dx > 0.0 {
                        (current_idx + 1) % visible_tabs.len()
                    } else if current_idx == 0 {
                        visible_tabs.len() - 1
                    } else {
                        current_idx - 1
                    };
                    let next_tab = visible_tabs[next_idx];
                    on_switch(next_tab);
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            } else {
                glib::Propagation::Proceed
            }
        });
        nav.container.add_controller(scroll_ctrl);

        nav
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    #[allow(dead_code)]
    pub fn tabs(&self) -> &[String] {
        &self.tabs_order
    }

    pub fn set_active_tab(&self, tab_id: &str) {
        *self.active_tab.borrow_mut() = tab_id.to_string();
        let palette = self.palette.borrow();
        for (id, btn) in &self.buttons {
            let icon_name = if id == "__settings__" {
                "settings"
            } else {
                let manifest = self.manifests.iter().find(|m| &m.id == id);
                manifest
                    .map(|m| {
                        if !m.icon.is_empty() {
                            m.icon.as_str()
                        } else {
                            m.id.as_str()
                        }
                    })
                    .unwrap_or(id.as_str())
            };

            if id == tab_id {
                btn.add_css_class("active");
                icons::set_button_icon(btn, icon_name, &palette.accent_color, 20);
            } else {
                btn.remove_css_class("active");
                icons::set_button_icon(btn, icon_name, &palette.text_secondary, 20);
            }
        }
    }

    pub fn set_tab_visible(&self, tab_id: &str, visible: bool) {
        for (id, btn) in &self.buttons {
            if id == tab_id {
                btn.set_visible(visible);
            }
        }
    }

    pub fn on_theme_changed(&self, new_palette: &ThemePalette) {
        *self.palette.borrow_mut() = new_palette.clone();
        let active = self.active_tab.borrow().clone();
        self.set_active_tab(&active);
    }
}
