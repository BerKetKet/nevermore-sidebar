use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, Orientation, PolicyType, ScrolledWindow, TextBuffer, TextView,
};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

pub struct NotepadPane {
    container: GtkBox,
    status_label: Label,
    buffer: TextBuffer,
    file_path: PathBuf,
    debounce_source: RefCell<Option<glib::SourceId>>,
}

impl NotepadPane {
    pub fn new(file_path: PathBuf) -> Rc<Self> {
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

        // Header Action Bar
        let header_bar = GtkBox::new(Orientation::Horizontal, 6);
        let header_title = Label::new(Some("SCRATCHPAD"));
        header_title.add_css_class("notepad-header-title");
        header_title.set_hexpand(true);
        header_title.set_xalign(0.0);

        let btn_clear = Button::with_label("Clear");
        btn_clear.add_css_class("btn-notepad-clear");

        header_bar.append(&header_title);
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

        // Footer status bar
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
            buffer: buffer.clone(),
            file_path,
            debounce_source: RefCell::new(None),
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

                let text = {
                    let start = buf.start_iter();
                    let end = buf.end_iter();
                    buf.text(&start, &end, false).to_string()
                };

                let p_timer = p.clone();
                let source_id =
                    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                        p_timer.debounce_source.borrow_mut().take();
                        p_timer.save_to_disk(&text);
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
            .set_label(&format!("{} words  |  {} chars", word_count, char_count));
    }

    fn save_to_disk(&self, content: &str) {
        let temp_path = self.file_path.with_extension("tmp");
        if let Ok(()) = fs::write(&temp_path, content) {
            let _ = fs::rename(&temp_path, &self.file_path);
        }
    }
}
