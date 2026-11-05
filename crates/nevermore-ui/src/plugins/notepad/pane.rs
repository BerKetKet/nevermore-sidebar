use crate::icons;
use crate::plugin_api::PluginContext;
use gtk4::accessible::Property;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, Orientation, PolicyType, ScrolledWindow, TextBuffer, TextView,
};
use nevermore_core::theme::ThemePalette;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotepadStore {
    pub active_id: String,
    pub notes: Vec<NoteItem>,
}

impl Default for NotepadStore {
    fn default() -> Self {
        let id = "note-1".to_string();
        Self {
            active_id: id.clone(),
            notes: vec![NoteItem {
                id,
                title: "New Note".to_string(),
                content: String::new(),
                updated_at: glib::DateTime::now_local()
                    .map(|d| d.to_unix())
                    .unwrap_or(0),
            }],
        }
    }
}

pub struct NotepadPane {
    container: GtkBox,
    status_label: Label,
    switcher_box: GtkBox,
    text_view: TextView,
    buffer: TextBuffer,
    btn_new: Button,
    btn_paste: Button,
    btn_copy: Button,
    btn_delete: Button,
    delete_confirming: Rc<Cell<bool>>,
    is_switching: Rc<Cell<bool>>,
    store: Rc<RefCell<NotepadStore>>,
    notes_json_path: PathBuf,
    debounce_source: RefCell<Option<glib::SourceId>>,
    palette: RefCell<ThemePalette>,
    ctx: PluginContext,
}

impl NotepadPane {
    pub fn new(ctx: &PluginContext) -> Rc<Self> {
        let theme = ctx.theme();
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_hexpand(true);
        container.set_vexpand(true);

        let notes_json_path = ctx.data_dir.join("notes.json");
        let store = if notes_json_path.exists() {
            fs::read_to_string(&notes_json_path)
                .ok()
                .and_then(|s| serde_json::from_str::<NotepadStore>(&s).ok())
                .unwrap_or_default()
        } else {
            // Check legacy notes.txt
            let notes_txt_path = ctx.data_dir.join("notes.txt");
            let legacy_text = if notes_txt_path.exists() {
                fs::read_to_string(&notes_txt_path).unwrap_or_default()
            } else {
                let legacy_path = std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        let home = std::env::var_os("HOME").unwrap_or_default();
                        PathBuf::from(home).join(".local").join("share")
                    })
                    .join("nevermore")
                    .join("notes.txt");
                if legacy_path.exists() {
                    fs::read_to_string(&legacy_path).unwrap_or_default()
                } else {
                    String::new()
                }
            };

            let trimmed = legacy_text.trim();
            let initial_note = if !trimmed.is_empty() {
                let first_line = trimmed
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("Note 1");
                let derived: String = first_line.trim().chars().take(18).collect();
                NoteItem {
                    id: "note-1".to_string(),
                    title: if derived.is_empty() {
                        "Note 1".to_string()
                    } else {
                        derived
                    },
                    content: legacy_text,
                    updated_at: glib::DateTime::now_local()
                        .map(|d| d.to_unix())
                        .unwrap_or(0),
                }
            } else {
                NoteItem {
                    id: "note-1".to_string(),
                    title: "New Note".to_string(),
                    content: String::new(),
                    updated_at: glib::DateTime::now_local()
                        .map(|d| d.to_unix())
                        .unwrap_or(0),
                }
            };

            let initial_store = NotepadStore {
                active_id: initial_note.id.clone(),
                notes: vec![initial_note],
            };
            if let Ok(json_str) = serde_json::to_string_pretty(&initial_store) {
                let _ = fs::write(&notes_json_path, json_str);
            }
            initial_store
        };

        let store_rc = Rc::new(RefCell::new(store));

        // 1. Top Action Bar: Left (New, Paste, Copy, Delete), Right (Word/Char counts)
        let header_bar = GtkBox::new(Orientation::Horizontal, 6);

        let btn_new = Button::new();
        btn_new.add_css_class("icon-btn");
        btn_new.set_tooltip_text(Some("New Note"));
        btn_new.update_property(&[Property::Label("New Note")]);
        icons::set_button_icon(&btn_new, "plus", &theme.text_secondary, 16);

        let btn_paste = Button::new();
        btn_paste.add_css_class("icon-btn");
        btn_paste.set_tooltip_text(Some("Paste from Clipboard"));
        btn_paste.update_property(&[Property::Label("Paste from Clipboard")]);
        icons::set_button_icon(&btn_paste, "list-plus", &theme.text_secondary, 16);

        let btn_copy = Button::new();
        btn_copy.add_css_class("icon-btn");
        btn_copy.set_tooltip_text(Some("Copy Note"));
        btn_copy.update_property(&[Property::Label("Copy Note")]);
        icons::set_button_icon(&btn_copy, "copy", &theme.text_secondary, 16);

        let btn_delete = Button::new();
        btn_delete.add_css_class("icon-btn");
        btn_delete.set_tooltip_text(Some("Delete Note"));
        btn_delete.update_property(&[Property::Label("Delete Note")]);
        icons::set_button_icon(&btn_delete, "trash-2", &theme.text_secondary, 16);

        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let status_label = Label::new(None);
        status_label.add_css_class("notepad-status");
        status_label.add_css_class("nm-data");
        status_label.set_xalign(1.0);

        header_bar.append(&btn_new);
        header_bar.append(&btn_paste);
        header_bar.append(&btn_copy);
        header_bar.append(&btn_delete);
        header_bar.append(&spacer);
        header_bar.append(&status_label);

        // 2. Note Switcher Bar (Horizontal scrollable bar of compact note pills)
        let switcher_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Never)
            .hexpand(true)
            .build();
        switcher_scroll.set_size_request(-1, 32);

        let switcher_box = GtkBox::new(Orientation::Horizontal, 4);
        switcher_scroll.set_child(Some(&switcher_box));

        // 3. Multi-line Text View
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

        let buffer = text_view.buffer();
        scrolled.set_child(Some(&text_view));

        container.append(&header_bar);
        container.append(&switcher_scroll);
        container.append(&scrolled);

        let delete_confirming = Rc::new(Cell::new(false));
        let is_switching = Rc::new(Cell::new(false));

        let pane = Rc::new(Self {
            container,
            status_label,
            switcher_box,
            text_view,
            buffer: buffer.clone(),
            btn_new: btn_new.clone(),
            btn_paste: btn_paste.clone(),
            btn_copy: btn_copy.clone(),
            btn_delete: btn_delete.clone(),
            delete_confirming,
            is_switching: is_switching.clone(),
            store: store_rc,
            notes_json_path,
            debounce_source: RefCell::new(None),
            palette: RefCell::new(theme),
            ctx: ctx.clone(),
        });

        // Initialize active note text into buffer
        {
            let store = pane.store.borrow();
            if let Some(active_note) = store.notes.iter().find(|n| n.id == store.active_id) {
                buffer.set_text(&active_note.content);
            } else if let Some(first_note) = store.notes.first() {
                buffer.set_text(&first_note.content);
            }
        }

        pane.update_counts(&buffer);
        pane.rebuild_switcher();

        // Connect New Note button
        let pane_new = pane.clone();
        btn_new.connect_clicked(move |_| {
            pane_new.create_new_note();
        });

        // Connect Paste button
        let pane_paste = pane.clone();
        btn_paste.connect_clicked(move |_| {
            pane_paste.paste_from_clipboard();
        });

        // Connect Copy button
        let pane_copy = pane.clone();
        btn_copy.connect_clicked(move |_| {
            pane_copy.copy_note();
        });

        // Connect Delete button
        let pane_del = pane.clone();
        btn_delete.connect_clicked(move |_| {
            pane_del.delete_current_note();
        });

        // Connect Text Buffer Change with debounced auto-save (500ms)
        let pane_weak = Rc::downgrade(&pane);
        buffer.connect_changed(move |buf| {
            if let Some(p) = pane_weak.upgrade() {
                if p.is_switching.get() {
                    return;
                }

                p.update_counts(buf);

                if let Some(src) = p.debounce_source.borrow_mut().take() {
                    src.remove();
                }

                let p_deb = p.clone();
                let buf_clone = buf.clone();
                let source_id = p.ctx.after_delay(Duration::from_millis(500), move || {
                    let start = buf_clone.start_iter();
                    let end = buf_clone.end_iter();
                    let text = buf_clone.text(&start, &end, false).to_string();

                    let active_id = p_deb.store.borrow().active_id.clone();
                    let mut title_changed = false;
                    {
                        let mut store = p_deb.store.borrow_mut();
                        if let Some(note) = store.notes.iter_mut().find(|n| n.id == active_id) {
                            note.content = text;
                            let first_line = note
                                .content
                                .lines()
                                .find(|l| !l.trim().is_empty())
                                .unwrap_or("Untitled Note");
                            let derived: String = first_line.trim().chars().take(18).collect();
                            let new_title = if derived.is_empty() {
                                "Untitled Note".to_string()
                            } else {
                                derived
                            };
                            if note.title != new_title {
                                note.title = new_title;
                                title_changed = true;
                            }
                            note.updated_at = glib::DateTime::now_local()
                                .map(|d| d.to_unix())
                                .unwrap_or(0);
                        }
                    }

                    p_deb.save_store_to_disk();
                    if title_changed {
                        p_deb.rebuild_switcher();
                    }
                    *p_deb.debounce_source.borrow_mut() = None;
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
        icons::set_button_icon(&self.btn_new, "plus", &theme.text_secondary, 16);
        icons::set_button_icon(&self.btn_paste, "list-plus", &theme.text_secondary, 16);
        icons::set_button_icon(&self.btn_copy, "copy", &theme.text_secondary, 16);
        let del_color = if self.delete_confirming.get() {
            "#ff5555"
        } else {
            &theme.text_secondary
        };
        icons::set_button_icon(&self.btn_delete, "trash-2", del_color, 16);
    }

    pub fn rebuild_switcher(self: &Rc<Self>) {
        while let Some(child) = self.switcher_box.first_child() {
            self.switcher_box.remove(&child);
        }

        let store = self.store.borrow();
        for note in &store.notes {
            let btn = Button::with_label(&note.title);
            btn.add_css_class("note-pill");
            btn.add_css_class("nm-data");
            if note.id == store.active_id {
                btn.add_css_class("active");
            }

            let note_id = note.id.clone();
            let pane_weak = Rc::downgrade(self);
            btn.connect_clicked(move |_| {
                if let Some(pane) = pane_weak.upgrade() {
                    pane.switch_to_note(&note_id);
                }
            });

            self.switcher_box.append(&btn);
        }
    }

    pub fn switch_to_note(self: &Rc<Self>, note_id: &str) {
        if self.store.borrow().active_id == note_id {
            return;
        }

        self.flush_save();

        let content = {
            let mut store = self.store.borrow_mut();
            store.active_id = note_id.to_string();
            store
                .notes
                .iter()
                .find(|n| n.id == note_id)
                .map(|n| n.content.clone())
                .unwrap_or_default()
        };

        self.is_switching.set(true);
        self.buffer.set_text(&content);
        self.is_switching.set(false);

        self.update_counts(&self.buffer);
        self.rebuild_switcher();
        self.save_store_to_disk();
    }

    pub fn create_new_note(self: &Rc<Self>) {
        self.flush_save();

        let now_unix = glib::DateTime::now_local()
            .map(|d| d.to_unix())
            .unwrap_or(0);
        let new_id = format!("note-{}", now_unix);
        let new_note = NoteItem {
            id: new_id.clone(),
            title: "New Note".to_string(),
            content: String::new(),
            updated_at: now_unix,
        };

        {
            let mut store = self.store.borrow_mut();
            store.notes.push(new_note);
            store.active_id = new_id;
        }

        self.is_switching.set(true);
        self.buffer.set_text("");
        self.is_switching.set(false);

        self.update_counts(&self.buffer);
        self.rebuild_switcher();
        self.save_store_to_disk();
        self.text_view.grab_focus();
    }

    pub fn paste_from_clipboard(&self) {
        if let Some(display) = gdk::Display::default() {
            let clipboard = display.clipboard();
            let buf = self.buffer.clone();
            clipboard.read_text_async(None::<&gio::Cancellable>, move |res| {
                if let Ok(Some(text)) = res {
                    buf.insert_at_cursor(&text);
                }
            });
        }
    }

    pub fn copy_note(&self) {
        let text = if let Some((start, end)) = self.buffer.selection_bounds() {
            self.buffer.text(&start, &end, false).to_string()
        } else {
            let start = self.buffer.start_iter();
            let end = self.buffer.end_iter();
            self.buffer.text(&start, &end, false).to_string()
        };

        if !text.is_empty() {
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&text);
            }
        }
    }

    pub fn delete_current_note(self: &Rc<Self>) {
        if self.delete_confirming.get() {
            self.delete_confirming.set(false);
            let theme = self.palette.borrow();
            icons::set_button_icon(&self.btn_delete, "trash-2", &theme.text_secondary, 16);

            let active_id = self.store.borrow().active_id.clone();
            let mut next_active_id = None;

            {
                let mut store = self.store.borrow_mut();
                if store.notes.len() > 1 {
                    if let Some(pos) = store.notes.iter().position(|n| n.id == active_id) {
                        store.notes.remove(pos);
                        let new_idx = if pos > 0 { pos - 1 } else { 0 };
                        store.active_id = store.notes[new_idx].id.clone();
                        next_active_id = Some(store.active_id.clone());
                    }
                } else if let Some(n) = store.notes.first_mut() {
                    n.content.clear();
                    n.title = "Untitled Note".to_string();
                    next_active_id = Some(n.id.clone());
                }
            }

            if let Some(nid) = next_active_id {
                let content = self
                    .store
                    .borrow()
                    .notes
                    .iter()
                    .find(|n| n.id == nid)
                    .map(|n| n.content.clone())
                    .unwrap_or_default();
                self.is_switching.set(true);
                self.buffer.set_text(&content);
                self.is_switching.set(false);
            }

            self.update_counts(&self.buffer);
            self.rebuild_switcher();
            self.save_store_to_disk();
        } else {
            self.delete_confirming.set(true);
            icons::set_button_icon(&self.btn_delete, "trash-2", "#ff5555", 16);
            let conf_clone = self.delete_confirming.clone();
            let btn_clone = self.btn_delete.clone();
            let pal_clone = self.palette.clone();
            self.ctx.after_delay(Duration::from_secs(3), move || {
                if conf_clone.get() {
                    conf_clone.set(false);
                    let theme = pal_clone.borrow();
                    icons::set_button_icon(&btn_clone, "trash-2", &theme.text_secondary, 16);
                }
            });
        }
    }

    /// Flushes any pending unsaved text immediately to disk (called on hide/shutdown).
    pub fn flush_save(&self) {
        if let Some(src) = self.debounce_source.borrow_mut().take() {
            src.remove();
        }

        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        let text = self.buffer.text(&start, &end, false).to_string();

        let active_id = self.store.borrow().active_id.clone();
        {
            let mut store = self.store.borrow_mut();
            if let Some(note) = store.notes.iter_mut().find(|n| n.id == active_id) {
                note.content = text;
                let first_line = note
                    .content
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("Untitled Note");
                let derived: String = first_line.trim().chars().take(18).collect();
                note.title = if derived.is_empty() {
                    "Untitled Note".to_string()
                } else {
                    derived
                };
                note.updated_at = glib::DateTime::now_local()
                    .map(|d| d.to_unix())
                    .unwrap_or(0);
            }
        }
        self.save_store_to_disk();
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

    fn save_store_to_disk(&self) {
        let store = self.store.borrow();
        if let Ok(json_str) = serde_json::to_string_pretty(&*store) {
            let temp_path = self.notes_json_path.with_extension("tmp");
            if fs::write(&temp_path, json_str).is_ok() {
                let _ = fs::rename(&temp_path, &self.notes_json_path);
            }
        }
    }
}
