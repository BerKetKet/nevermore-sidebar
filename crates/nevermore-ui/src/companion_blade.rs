use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, EventControllerMotion, Orientation, Widget,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub const COMPANION_BLADE_WIDTH: i32 = 500;
pub const PANEL_WIDTH: i32 = 400;

#[derive(Clone)]
pub struct CompanionBlade {
    window: ApplicationWindow,
    content_box: GtkBox,
    is_hovered: Rc<Cell<bool>>,
    active_plugin: Rc<RefCell<Option<String>>>,
}

impl CompanionBlade {
    pub fn new(app: &Application, monitor: &Option<gdk::Monitor>, monitor_height: i32) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Nevermore Companion Blade")
            .build();

        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_exclusive_zone(-1);
            window.set_anchor(Edge::Right, true);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, false);
            window.set_margin(Edge::Right, PANEL_WIDTH);
            window.set_margin(Edge::Top, 0);
            window.set_margin(Edge::Bottom, 0);
            window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);

            if let Some(ref mon) = monitor {
                window.set_monitor(Some(mon));
            }
        }

        window.set_size_request(COMPANION_BLADE_WIDTH, monitor_height);
        window.set_default_size(COMPANION_BLADE_WIDTH, monitor_height);
        window.set_resizable(false);
        window.add_css_class("sidebar-panel");
        window.add_css_class("companion-blade");

        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.set_size_request(COMPANION_BLADE_WIDTH, monitor_height);
        content_box.set_vexpand(true);
        content_box.set_hexpand(true);
        window.set_child(Some(&content_box));

        let is_hovered = Rc::new(Cell::new(false));
        let active_plugin = Rc::new(RefCell::new(None));

        let blade_motion = EventControllerMotion::new();
        let hovered_enter = is_hovered.clone();
        blade_motion.connect_enter(move |_, _, _| {
            hovered_enter.set(true);
        });

        let hovered_leave = is_hovered.clone();
        blade_motion.connect_leave(move |_| {
            hovered_leave.set(false);
        });

        window.add_controller(blade_motion);

        Self {
            window,
            content_box,
            is_hovered,
            active_plugin,
        }
    }

    pub fn for_test() -> Self {
        let window = ApplicationWindow::builder().build();
        let content_box = GtkBox::new(Orientation::Vertical, 0);
        Self {
            window,
            content_box,
            is_hovered: Rc::new(Cell::new(false)),
            active_plugin: Rc::new(RefCell::new(None)),
        }
    }

    pub fn is_hovered(&self) -> bool {
        self.is_hovered.get()
    }

    pub fn set_hovered(&self, hovered: bool) {
        self.is_hovered.set(hovered);
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn is_open(&self) -> bool {
        self.is_visible()
    }

    pub fn open(&self, plugin_id: &str, widget: &Widget) {
        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }
        self.content_box.append(widget);
        *self.active_plugin.borrow_mut() = Some(plugin_id.to_string());
        self.window.set_visible(true);
    }

    pub fn close(&self) {
        if self.window.is_visible() {
            self.window.set_visible(false);
        }
        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }
        *self.active_plugin.borrow_mut() = None;
    }

    pub fn hide_if_plugin_differs(&self, current_plugin: &str) {
        if let Some(ref active) = *self.active_plugin.borrow() {
            if active != current_plugin {
                self.close();
            }
        }
    }

    pub fn window(&self) -> &ApplicationWindow {
        &self.window
    }
}
