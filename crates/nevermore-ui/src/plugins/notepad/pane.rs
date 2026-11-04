use crate::icons;
use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, Orientation, PolicyType, ScrolledWindow, TextBuffer, TextView,
};
use nevermore_core::theme::ThemePalette;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

pub struct NotepadPane {
    container: GtkBox,
    status_label: Label,
    btn_clear: Button,
    buffer: TextBuffer,
    file_path: PathBuf,
    debounce_source: RefCell<Option<glib::SourceId>>,
    palette: RefCell<ThemePalette>,
}

impl NotepadPane {
    pub fn new(file_path: PathBuf, theme: &ThemePalette) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_hexpand(true);
        container.set_vexpand(true);

        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Migrate from legacy $XDG_DATA_HOME/nevermore/notes.txt if exists and target is empty
        if !file_path.exists() {
            let legacy_path = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    let home = env::var_os("HOME").unwrap_or_default();
                    PathBuf::from(home).join(".local").join("share")
                })
                .join("nevermore")
                .join("notes.txt");
            if legacy_path.exists() {
                let _ = fs::copy(&legacy_path, &file_path);
            }
        }

        // Header Action Bar (zero static title text, Lucide eraser icon)
        let header_bar = GtkBox::new(Orientation::Horizontal, 6);
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let btn_clear = Button::new();
        btn_clear.add_css_class("btn-notepad-clear");
        btn_clear.set_tooltip_text(Some("Clear Notepad"));
        btn_clear.update_property(&[Property::Label("Clear Notepad")]);
        icons::set_button_icon(&btn_clear, "eraser", &theme.text_secondary, 16);

        header_bar.append(&spacer);
        header_bar.append(&btn_clear);

        // Multi-line Text View
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .build();
        scrolled.add_css_class("notepad-scroll");

        let text_view = TextView::new();
        text_view.add_css_class("notepad-view");
        text_view.set_wrap_mode(gtk4::WrapMode::Word);
        text_view.set_monospace(true);

        // Load existing content from disk
        let initial_text = fs::read_to_string(&file_path).unwrap_or_default();
        let buffer = text_view.buffer();
        buffer.set_text(&initial_text);

        scrolled.set_child(Some(&text_view));

        // Footer status bar (compact numeric data only, e.g. "12w · 64c")
        let footer = GtkBox::new(Orientation::Horizontal, 8);
        let status_label = Label::new(None);
        status_label.add_css_class("notepad-status");
        status_label.set_hexpand(true);
        status_label.set_xalign(1.0);
        footer.append(&status_label);

        container.append(&header_bar);
        container.append(&scrolled);
        container.append(&footer);

        let pane = Rc::new(Self {
            container,
            status_label,
            btn_clear: btn_clear.clone(),
            buffer: buffer.clone(),
            file_path,
            debounce_source: RefCell::new(None),
            palette: RefCell::new(theme.clone()),
        });

        pane.update_counts(&buffer);

        // Connect Clear button
        let buf_clear = buffer.clone();
        btn_clear.connect_clicked(move |_| {
            buf_clear.set_text("");
        });

        // Connect text buffer changed with atomic auto-save debounce (500ms)
        let pane_weak = Rc::downgrade(&pane);
        buffer.connect_changed(move |buf| {
            if let Some(p) = pane_weak.upgrade() {
                p.update_counts(buf);

                // Cancel existing pending debounce
                if let Some(src) = p.debounce_source.borrow_mut().take() {
                    src.remove();
                }

                let p_debounce = p.clone();
                let buf_clone = buf.clone();
                let source_id =
                    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                        let start = buf_clone.start_iter();
                        let end = buf_clone.end_iter();
                        let text = buf_clone.text(&start, &end, false);
                        p_debounce.save_to_disk(&text);
                        *p_debounce.debounce_source.borrow_mut() = None;
                        glib::ControlFlow::Break
                    });
                *p.debounce_source.borrow_mut() = Some(source_id);
            }
        });

        pane
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn apply_theme(&self, theme: &ThemePalette) {
        *self.palette.borrow_mut() = theme.clone();
        icons::set_button_icon(&self.btn_clear, "eraser", &theme.text_secondary, 16);
    }

    /// Flushes any pending unsaved text immediately to disk (called on hide/shutdown).
    pub fn flush_save(&self) {
        if let Some(src) = self.debounce_source.borrow_mut().take() {
            src.remove();
            let start = self.buffer.start_iter();
            let end = self.buffer.end_iter();
            let text = self.buffer.text(&start, &end, false);
            self.save_to_disk(&text);
        }
    }

    fn update_counts(&self, buffer: &TextBuffer) {
        let char_count = buffer.char_count();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, false);
        let word_count = text.split_whitespace().count();

        self.status_label
            .set_label(&format!("{}w · {}c", word_count, char_count));
    }

    fn save_to_disk(&self, content: &str) {
        let temp_path = self.file_path.with_extension("tmp");
        if let Ok(()) = fs::write(&temp_path, content) {
            let _ = fs::rename(&temp_path, &self.file_path);
        }
    }
}
