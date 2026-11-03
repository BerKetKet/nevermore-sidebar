use gtk4::gdk;
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

fn icon_for_tab(id: &str) -> &'static [u8] {
    match id {
        "terminal" => ICON_TERMINAL,
        "widgets" => ICON_WIDGETS,
        "notifications" => ICON_NOTIFICATIONS,
        "notepad" => ICON_NOTEPAD,
        "media" => ICON_MEDIA,
        "gemini" => ICON_GEMINI,
        "youtube" => ICON_YOUTUBE,
        "vpn" => ICON_VPN,
        "email" => ICON_EMAIL,
        _ => ICON_TERMINAL,
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

            let bytes = glib::Bytes::from_static(icon_for_tab(id));
            if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
                let img = Image::from_paintable(Some(&texture));
                img.set_pixel_size(20);
                btn.set_child(Some(&img));
            } else {
                let label = if id.len() >= 4 { &id[..4] } else { id.as_str() };
                btn.set_label(&label.to_uppercase());
            }

            if id == "terminal" {
                btn.add_css_class("active");
            }

            let tab_id = id.clone();
            let on_switch = on_tab_switched.clone();
            let act_ref = active_tab.clone();

            btn.connect_clicked(move |_| {
                *act_ref.borrow_mut() = tab_id.clone();
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

        // Horizontal swipe gesture on Nav Rail
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
                    *n.active_tab.borrow_mut() = next_tab.clone();
                    n.set_active_tab(next_tab);
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
