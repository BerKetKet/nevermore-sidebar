use gtk4::gdk;
use gtk4::pango;
use gtk4::prelude::*;
use gtk4::{gio, glib};
use gtk4::{Box as GtkBox, Button, Label, Orientation, PolicyType, ScrolledWindow, Stack};
use nevermore_core::theme::ThemePalette;
use nevermore_core::traits::{SessionBackend, TaskBackend};
use nevermore_core::{SystemdTaskBackend, TmuxBackend};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use vte4::prelude::*;
use vte4::Terminal;

struct TerminalPage {
    id: String,
    title: String,
    session_name: String,
    terminal: Terminal,
}

pub struct TerminalPane {
    container: GtkBox,
    page_bar_box: GtkBox,
    stack: Stack,
    bg_list_box: GtkBox,
    pages: Rc<RefCell<Vec<TerminalPage>>>,
    active_page_id: Rc<RefCell<String>>,
    tmux: TmuxBackend,
    systemd: SystemdTaskBackend,
    self_weak: RefCell<Option<Weak<TerminalPane>>>,
    current_theme: RefCell<ThemePalette>,
}

impl TerminalPane {
    pub fn new(tmux: TmuxBackend, systemd: SystemdTaskBackend, theme: &ThemePalette) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // 1. Page Bar with ScrolledWindow to prevent width expansion
        let page_bar = GtkBox::new(Orientation::Horizontal, 4);
        page_bar.add_css_class("term-page-bar");

        let page_bar_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Never)
            .hexpand(true)
            .build();

        let page_bar_box = GtkBox::new(Orientation::Horizontal, 4);
        page_bar_scroll.set_child(Some(&page_bar_box));

        let btn_new = Button::with_label("+");
        btn_new.set_tooltip_text(Some("New Terminal Page"));
        btn_new.add_css_class("btn-term-ctrl");

        let btn_bg = Button::with_label("⏏");
        btn_bg.set_tooltip_text(Some("Send to Background"));
        btn_bg.add_css_class("btn-term-ctrl");

        let btn_close = Button::with_label("✕");
        btn_close.set_tooltip_text(Some("Close Page"));
        btn_close.add_css_class("btn-term-ctrl");

        page_bar.append(&page_bar_scroll);
        page_bar.append(&btn_new);
        page_bar.append(&btn_bg);
        page_bar.append(&btn_close);

        // 2. Terminal Viewport Stack
        let stack = Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.add_css_class("term-viewport");

        // 3. Background Sessions Section (collapsible / scrollable)
        let bg_section = GtkBox::new(Orientation::Vertical, 4);
        bg_section.add_css_class("bg-sessions-section");
        bg_section.set_size_request(-1, 140);

        let bg_header = Label::new(Some("BACKGROUND SESSIONS & TASKS"));
        bg_header.add_css_class("bg-sessions-header");
        bg_header.set_xalign(0.0);
        bg_section.append(&bg_header);

        let bg_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .min_content_height(100)
            .build();

        let bg_list_box = GtkBox::new(Orientation::Vertical, 4);
        bg_scroll.set_child(Some(&bg_list_box));
        bg_section.append(&bg_scroll);

        container.append(&page_bar);
        container.append(&stack);
        container.append(&bg_section);

        let pane = Rc::new(Self {
            container,
            page_bar_box,
            stack,
            bg_list_box,
            pages: Rc::new(RefCell::new(Vec::new())),
            active_page_id: Rc::new(RefCell::new(String::new())),
            tmux,
            systemd,
            self_weak: RefCell::new(None),
            current_theme: RefCell::new(theme.clone()),
        });

        *pane.self_weak.borrow_mut() = Some(Rc::downgrade(&pane));

        // Connect action buttons
        let pane_clone = pane.clone();
        btn_new.connect_clicked(move |_| {
            pane_clone.create_new_page(None);
        });

        let pane_clone = pane.clone();
        btn_bg.connect_clicked(move |_| {
            pane_clone.send_active_to_background();
        });

        let pane_clone = pane.clone();
        btn_close.connect_clicked(move |_| {
            pane_clone.close_active_page();
        });

        // Auto-restore foreground terminal pages on startup/restart
        let existing = pane.tmux.list_sessions().unwrap_or_default();
        let mut foreground_opened = 0;
        for s in &existing {
            if !pane.tmux.is_session_backgrounded(&s.name) {
                pane.create_new_page(Some(&s.name));
                foreground_opened += 1;
            }
        }
        if foreground_opened == 0 {
            pane.create_new_page(Some("main"));
        }
        pane.refresh_background_list();

        pane
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn apply_theme(&self, theme: &ThemePalette) {
        *self.current_theme.borrow_mut() = theme.clone();

        let fg = gdk::RGBA::parse(&theme.term_fg).unwrap_or(gdk::RGBA::new(0.96, 0.96, 0.96, 1.0));
        let bg =
            gdk::RGBA::parse(&theme.term_bg).unwrap_or(gdk::RGBA::new(0.094, 0.094, 0.106, 1.0));
        let palette: Vec<gdk::RGBA> = theme
            .term_palette
            .iter()
            .filter_map(|c| gdk::RGBA::parse(c).ok())
            .collect();

        let font_desc = format!("{} {}", theme.mono_font, theme.mono_font_size);
        let pango_font = pango::FontDescription::from_string(&font_desc);

        for p in self.pages.borrow().iter() {
            let pal_refs: Vec<&gdk::RGBA> = palette.iter().collect();
            p.terminal.set_colors(Some(&fg), Some(&bg), &pal_refs);
            p.terminal.set_font(Some(&pango_font));
        }
    }

    /// Spawns a new terminal page attached to tmux -L nevermore with -f <conf_path>.
    pub fn create_new_page(&self, custom_name: Option<&str>) {
        let count = self.pages.borrow().len() + 1;
        let session_name = match custom_name {
            Some(name) => name.to_string(),
            None => format!("page_{}", count),
        };
        let page_id = format!("page_{}_{}", session_name, glib::random_int());
        let title = format!("{}: {}", count, session_name);

        let terminal = Terminal::new();
        terminal.set_hexpand(true);
        terminal.set_vexpand(true);
        terminal.set_size_request(200, 200);
        terminal.set_scrollback_lines(50000);

        // Apply theme font and palette
        let theme = self.current_theme.borrow();
        let font_desc = format!("{} {}", theme.mono_font, theme.mono_font_size);
        terminal.set_font(Some(&pango::FontDescription::from_string(&font_desc)));

        let fg = gdk::RGBA::parse(&theme.term_fg).unwrap_or(gdk::RGBA::new(0.96, 0.96, 0.96, 1.0));
        let bg =
            gdk::RGBA::parse(&theme.term_bg).unwrap_or(gdk::RGBA::new(0.094, 0.094, 0.106, 1.0));
        let palette: Vec<gdk::RGBA> = theme
            .term_palette
            .iter()
            .filter_map(|c| gdk::RGBA::parse(c).ok())
            .collect();
        let pal_refs: Vec<&gdk::RGBA> = palette.iter().collect();
        terminal.set_colors(Some(&fg), Some(&bg), &pal_refs);

        // Spawn tmux session with -f <conf_path>
        let conf_str = self.tmux.conf_path().to_string_lossy().to_string();
        let tmux_args = [
            "/usr/bin/tmux",
            "-L",
            self.tmux.socket_name(),
            "-f",
            &conf_str,
            "new-session",
            "-A",
            "-s",
            &session_name,
        ];

        terminal.spawn_async(
            vte4::PtyFlags::DEFAULT,
            None,
            &tmux_args,
            &[],
            glib::SpawnFlags::DEFAULT,
            || (),
            -1,
            gio::Cancellable::NONE,
            move |_| {},
        );

        let _ = self.tmux.set_session_backgrounded(&session_name, false);

        let page = TerminalPage {
            id: page_id.clone(),
            title,
            session_name,
            terminal: terminal.clone(),
        };

        self.stack.add_named(&terminal, Some(&page_id));
        self.pages.borrow_mut().push(page);
        self.switch_to_page(&page_id);
        self.rebuild_page_bar();
        self.refresh_background_list();
    }

    pub fn switch_to_page(&self, page_id: &str) {
        *self.active_page_id.borrow_mut() = page_id.to_string();
        self.stack.set_visible_child_name(page_id);
        self.rebuild_page_bar();
    }

    pub fn send_active_to_background(&self) {
        let active_id = self.active_page_id.borrow().clone();
        if active_id.is_empty() {
            return;
        }

        let mut pages = self.pages.borrow_mut();
        if let Some(pos) = pages.iter().position(|p| p.id == active_id) {
            let removed = pages.remove(pos);
            self.stack.remove(&removed.terminal);
            let _ = self
                .tmux
                .set_session_backgrounded(&removed.session_name, true);

            // Switch to next remaining page or clear active
            if let Some(next) = pages.first() {
                let next_id = next.id.clone();
                drop(pages);
                self.switch_to_page(&next_id);
            } else {
                drop(pages);
                *self.active_page_id.borrow_mut() = String::new();
            }
        }
        self.rebuild_page_bar();
        self.refresh_background_list();
    }

    pub fn close_active_page(&self) {
        let active_id = self.active_page_id.borrow().clone();
        if active_id.is_empty() {
            return;
        }

        let mut pages = self.pages.borrow_mut();
        if let Some(pos) = pages.iter().position(|p| p.id == active_id) {
            let removed = pages.remove(pos);
            self.stack.remove(&removed.terminal);
            let _ = self.tmux.kill_session(&removed.session_name);

            if let Some(next) = pages.first() {
                let next_id = next.id.clone();
                drop(pages);
                self.switch_to_page(&next_id);
            } else {
                drop(pages);
                *self.active_page_id.borrow_mut() = String::new();
            }
        }
        self.rebuild_page_bar();
        self.refresh_background_list();
    }

    pub fn reattach_session(self: &Rc<Self>, session_name: &str) {
        let _ = self.tmux.set_session_backgrounded(session_name, false);
        self.create_new_page(Some(session_name));
    }

    fn rebuild_page_bar(&self) {
        while let Some(child) = self.page_bar_box.first_child() {
            self.page_bar_box.remove(&child);
        }

        let active_id = self.active_page_id.borrow().clone();
        let pages = self.pages.borrow();

        for p in pages.iter() {
            let btn = Button::with_label(&p.title);
            btn.add_css_class("term-page-btn");
            if p.id == active_id {
                btn.add_css_class("active");
            }

            let pid = p.id.clone();
            let stack_ref = self.stack.clone();
            let act_ref = self.active_page_id.clone();
            let bar_box = self.page_bar_box.clone();

            btn.connect_clicked(move |b| {
                *act_ref.borrow_mut() = pid.clone();
                stack_ref.set_visible_child_name(&pid);

                // Update tab buttons active class
                let mut sib = bar_box.first_child();
                while let Some(w) = sib {
                    w.remove_css_class("active");
                    sib = w.next_sibling();
                }
                b.add_css_class("active");
            });

            self.page_bar_box.append(&btn);
        }
    }

    pub fn refresh_background_list(&self) {
        while let Some(child) = self.bg_list_box.first_child() {
            self.bg_list_box.remove(&child);
        }

        // List background tmux sessions
        if let Ok(sessions) = self.tmux.list_sessions() {
            for s in sessions {
                if self.tmux.is_session_backgrounded(&s.name) {
                    let row = GtkBox::new(Orientation::Horizontal, 6);
                    row.add_css_class("bg-session-row");

                    let lbl = Label::new(Some(&format!("tmux: {}", s.name)));
                    lbl.add_css_class("bg-session-label");
                    lbl.set_hexpand(true);
                    lbl.set_xalign(0.0);
                    row.append(&lbl);

                    let btn_attach = Button::with_label("Attach");
                    btn_attach.add_css_class("btn-bg-attach");

                    let s_name = s.name.clone();
                    let weak = self.self_weak.borrow().clone();
                    btn_attach.connect_clicked(move |_| {
                        if let Some(w) = &weak {
                            if let Some(pane_rc) = w.upgrade() {
                                pane_rc.reattach_session(&s_name);
                            }
                        }
                    });
                    row.append(&btn_attach);

                    let btn_kill = Button::with_label("✕");
                    btn_kill.add_css_class("btn-bg-kill");
                    let tmux_ref = self.tmux.clone();
                    let s_name_kill = s.name.clone();
                    let weak_kill = self.self_weak.borrow().clone();
                    btn_kill.connect_clicked(move |_| {
                        let _ = tmux_ref.kill_session(&s_name_kill);
                        if let Some(w) = &weak_kill {
                            if let Some(pane_rc) = w.upgrade() {
                                pane_rc.refresh_background_list();
                            }
                        }
                    });
                    row.append(&btn_kill);

                    self.bg_list_box.append(&row);
                }
            }
        }

        // List background transient systemd tasks
        if let Ok(tasks) = self.systemd.list_tasks() {
            for t in tasks {
                let row = GtkBox::new(Orientation::Horizontal, 6);
                row.add_css_class("bg-session-row");

                let lbl = Label::new(Some(&format!("task: {} ({:?})", t.command, t.state)));
                lbl.add_css_class("bg-session-label");
                lbl.set_hexpand(true);
                lbl.set_xalign(0.0);
                row.append(&lbl);

                let btn_kill = Button::with_label("Stop");
                btn_kill.add_css_class("btn-bg-kill");
                let sys_kill = self.systemd.clone();
                let unit_kill = t.unit.clone();
                let weak_kill = self.self_weak.borrow().clone();
                btn_kill.connect_clicked(move |_| {
                    let _ = sys_kill.stop_task(&unit_kill);
                    if let Some(w) = &weak_kill {
                        if let Some(pane_rc) = w.upgrade() {
                            pane_rc.refresh_background_list();
                        }
                    }
                });
                row.append(&btn_kill);

                self.bg_list_box.append(&row);
            }
        }
    }
}
