use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, EventControllerScroll, EventControllerScrollFlags, Image, Orientation,
};
use nevermore_core::plugin::PluginManifest;
use std::cell::RefCell;
use std::rc::Rc;

const ICON_TERMINAL: &[u8] = include_bytes!("../resources/icons/terminal.svg");
const ICON_WIDGETS: &[u8] = include_bytes!("../resources/icons/widgets.svg");
const ICON_NOTIFICATIONS: &[u8] = include_bytes!("../resources/icons/notifications.svg");
const ICON_NOTEPAD: &[u8] = include_bytes!("../resources/icons/notepad.svg");
const ICON_MEDIA: &[u8] = include_bytes!("../resources/icons/media.svg");
const ICON_GEMINI: &[u8] = include_bytes!("../resources/icons/gemini.svg");
const ICON_YOUTUBE: &[u8] = include_bytes!("../resources/icons/youtube.svg");
const ICON_VPN: &[u8] = include_bytes!("../resources/icons/vpn.svg");
const ICON_EMAIL: &[u8] = include_bytes!("../resources/icons/email.svg");

// Generic puzzle-piece fallback icon for external plugins without custom SVGs
const ICON_FALLBACK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="#f4f4f5" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19.439 7.85c0-1.571-.802-2.85-1.793-2.85s-1.793 1.279-1.793 2.85H12V3.561C12 2.7 10.72 2 9.15 2S6.3 2.7 6.3 3.561V7.85H2v3.85c1.571 0 2.85.802 2.85 1.793s-1.279 1.793-2.85 1.793V19h4.3v-4.289c0-.861 1.28-1.561 2.85-1.561s2.85.7 2.85 1.561V19h4.289c0-1.571.802-2.85 1.793-2.85s1.793 1.279 1.793 2.85H22v-4.3c-1.571 0-2.85-.802-2.85-1.793s1.279-1.793 2.85-1.793V7.85h-2.561z"/></svg>"##;

fn icon_for_manifest(manifest: &PluginManifest) -> &'static [u8] {
    let icon = manifest.icon.trim();
    let id = manifest.id.trim();

    if icon == "terminal.svg" || id == "terminal" {
        ICON_TERMINAL
    } else if icon == "widgets.svg" || id == "widgets" {
        ICON_WIDGETS
    } else if icon == "notifications.svg" || id == "notifications" {
        ICON_NOTIFICATIONS
    } else if icon == "notepad.svg" || id == "notepad" {
        ICON_NOTEPAD
    } else if icon == "media.svg" || id == "media" {
        ICON_MEDIA
    } else if icon == "gemini.svg" || id == "gemini" {
        ICON_GEMINI
    } else if icon == "youtube.svg" || id == "youtube" {
        ICON_YOUTUBE
    } else if icon == "vpn.svg" || id == "vpn" {
        ICON_VPN
    } else if icon == "email.svg" || id == "email" {
        ICON_EMAIL
    } else {
        ICON_FALLBACK
    }
}

pub struct NavRail {
    container: GtkBox,
    buttons: Vec<(String, Button)>,
    active_tab: Rc<RefCell<String>>,
    tabs_order: Vec<String>,
}

impl NavRail {
    pub fn new<F>(manifests: &[PluginManifest], on_tab_switched: F) -> Rc<Self>
    where
        F: Fn(&str) + 'static + Clone,
    {
        let container = GtkBox::new(Orientation::Vertical, 4);
        container.set_size_request(60, -1);
        container.set_hexpand(false);
        container.add_css_class("nav-rail");

        let active_tab = Rc::new(RefCell::new("terminal".to_string()));
        let mut buttons = Vec::new();
        let mut tabs_order = Vec::new();

        for manifest in manifests {
            let id = &manifest.id;
            tabs_order.push(id.clone());

            let btn = Button::new();
            btn.add_css_class("nav-btn");
            btn.set_tooltip_text(Some(&manifest.title));

            let bytes = glib::Bytes::from_static(icon_for_manifest(manifest));
            if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
                let img = Image::from_paintable(Some(&texture));
                img.set_pixel_size(20);
                btn.set_child(Some(&img));
            } else {
                // Safe character truncation: never slices across UTF-8 code point boundaries
                let label = id.chars().take(4).collect::<String>();
                btn.set_label(&label.to_uppercase());
            }

            if id == "terminal" {
                btn.add_css_class("active");
            }

            let tab_id = id.clone();
            let on_switch = on_tab_switched.clone();

            btn.connect_clicked(move |_| {
                on_switch(&tab_id);
            });

            container.append(&btn);
            buttons.push((id.clone(), btn));
        }

        let nav = Rc::new(Self {
            container,
            buttons,
            active_tab,
            tabs_order,
        });

        // Horizontal swipe gesture on Nav Rail (Fix 8 - Zone Rule)
        let scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::HORIZONTAL);
        let nav_weak = Rc::downgrade(&nav);
        let on_switch = on_tab_switched;
        scroll_ctrl.connect_scroll(move |_, dx, _| {
            if let Some(n) = nav_weak.upgrade() {
                if dx.abs() > 0.5 {
                    let cur = n.active_tab.borrow().clone();
                    let current_idx = n.tabs_order.iter().position(|id| *id == cur).unwrap_or(0);
                    let next_idx = if dx > 0.0 {
                        (current_idx + 1) % n.tabs_order.len()
                    } else if current_idx == 0 {
                        n.tabs_order.len() - 1
                    } else {
                        current_idx - 1
                    };
                    let next_tab = &n.tabs_order[next_idx];
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
        for (id, btn) in &self.buttons {
            if id == tab_id {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
    }
}
