extern crate self as nevermore_ui;

pub mod icons;
pub mod ipc_server;
pub mod nav_rail;
pub mod plugin_api;
pub mod plugins;
pub mod prelude;
pub mod sdk;
pub mod settings_page;

pub use sdk::lifecycle::safe_source_remove;

/// Helper to detect open popovers / dropdown menus in a widget tree.
pub fn has_visible_popover(root: &impl gtk4::prelude::IsA<gtk4::Widget>) -> bool {
    use gtk4::prelude::*;
    fn check_widget(w: &gtk4::Widget) -> bool {
        if w.is_visible() && (w.is::<gtk4::Popover>() || w.is::<gtk4::PopoverMenu>()) {
            return true;
        }
        let mut child = w.first_child();
        while let Some(c) = child {
            if check_widget(&c) {
                return true;
            }
            child = c.next_sibling();
        }
        false
    }
    check_widget(root.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4::prelude::*;

    #[test]
    fn test_has_visible_popover_detection() {
        if gtk4::init().is_err() {
            eprintln!("Skipping popover test: headless without display");
            return;
        }

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        assert!(!has_visible_popover(&container));

        let popover = gtk4::Popover::new();
        popover.set_parent(&container);
        assert!(!has_visible_popover(&container));
        popover.unparent();
    }
}
