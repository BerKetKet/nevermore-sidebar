use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, EventControllerScroll, EventControllerScrollFlags, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

pub const TABS: &[(&str, &str)] = &[
    ("terminal", "TERM"),
    ("widgets", "WIDG"),
    ("notifications", "NOTI"),
    ("notes", "NOTE"),
    ("media", "MED"),
    ("ai", "AI"),
    ("youtube", "YT"),
    ("vpn", "VPN"),
    ("email", "MAIL"),
];

pub struct NavRail {
    container: GtkBox,
    buttons: Vec<(String, Button)>,
    active_tab: Rc<RefCell<String>>,
}

impl NavRail {
    pub fn new<F>(on_tab_switched: F) -> Rc<Self>
    where
        F: Fn(&str) + 'static + Clone,
    {
        let container = GtkBox::new(Orientation::Vertical, 4);
        container.set_size_request(60, -1);
        container.add_css_class("nav-rail");

        let active_tab = Rc::new(RefCell::new("terminal".to_string()));
        let mut buttons = Vec::new();

        for (id, label) in TABS {
            let btn = Button::with_label(label);
            btn.add_css_class("nav-btn");
            if *id == "terminal" {
                btn.add_css_class("active");
            }

            let tab_id = id.to_string();
            let on_switch = on_tab_switched.clone();
            let act_ref = active_tab.clone();

            btn.connect_clicked(move |_| {
                *act_ref.borrow_mut() = tab_id.clone();
                on_switch(&tab_id);
            });

            container.append(&btn);
            buttons.push((id.to_string(), btn));
        }

        let nav = Rc::new(Self {
            container,
            buttons,
            active_tab,
        });

        // 2-Finger Horizontal Touchpad Swipe (Zone Rule on Nav Rail)
        let scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::HORIZONTAL);
        let nav_clone = nav.clone();
        let on_switch = on_tab_switched.clone();

        scroll_ctrl.connect_scroll(move |_, dx, _| {
            if dx.abs() > 0.5 {
                let cur = nav_clone.active_tab.borrow().clone();
                let current_idx = TABS.iter().position(|(id, _)| *id == cur).unwrap_or(0);
                let next_idx = if dx > 0.0 {
                    (current_idx + 1) % TABS.len()
                } else if current_idx == 0 {
                    TABS.len() - 1
                } else {
                    current_idx - 1
                };
                let next_tab = TABS[next_idx].0;
                nav_clone.set_active_tab(next_tab);
                on_switch(next_tab);
                glib::Propagation::Stop
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

    pub fn set_active_tab(&self, tab: &str) {
        *self.active_tab.borrow_mut() = tab.to_string();
        for (id, btn) in &self.buttons {
            if id == tab {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
    }
}
