use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow, Stack};
use nevermore_core::tmux::TmuxBackend;
use nevermore_core::traits::{SessionBackend, TaskBackend};
use nevermore_core::SystemdTaskBackend;
use std::cell::RefCell;
use std::rc::Rc;
use vte4::prelude::*;
use vte4::Terminal;

#[derive(Clone)]
pub struct TerminalPage {
    pub id: String,
    pub title: String,
    pub session_name: String,
    pub terminal: Terminal,
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
}

impl TerminalPane {
    pub fn new() -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // 1. Page Bar
        let page_bar = GtkBox::new(Orientation::Horizontal, 4);
        page_bar.add_css_class("page-bar");

        let page_bar_box = GtkBox::new(Orientation::Horizontal, 4);
        page_bar_box.set_hexpand(true);

        let btn_new = Button::with_label("+ New");
        btn_new.add_css_class("btn-new-page");

        let btn_bg = Button::with_label("⏏ Background");
        btn_bg.add_css_class("btn-action-bg");

        let btn_close = Button::with_label("✕ Close");
        btn_close.add_css_class("btn-action-close");

        page_bar.append(&page_bar_box);
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
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
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
            tmux: TmuxBackend::new("nevermore"),
            systemd: SystemdTaskBackend::new("nevermore-task"),
        });

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

        // Initialize first default page
        pane.create_new_page(Some("main"));
        pane.refresh_background_list();

        pane
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Spawns a new terminal page attached to tmux -L nevermore.
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
        terminal.set_scrollback_lines(50000);

        // Solid dark theme colors
        let fg = gdk::RGBA::new(0.96, 0.96, 0.96, 1.0);
        let bg = gdk::RGBA::new(0.094, 0.094, 0.106, 1.0); // #18181b
        terminal.set_colors(Some(&fg), Some(&bg), &[]);

        // Spawn tmux session
        let tmux_args = [
            "/usr/bin/tmux",
            "-L",
            self.tmux.socket_name(),
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

            // Switch to next remaining page or create new if empty
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

    fn rebuild_page_bar(&self) {
        while let Some(child) = self.page_bar_box.first_child() {
            self.page_bar_box.remove(&child);
        }

        let pages = self.pages.borrow();
        let active_id = self.active_page_id.borrow().clone();

        for p in pages.iter() {
            let btn = Button::with_label(&p.title);
            btn.add_css_class("page-tab-btn");
            if p.id == active_id {
                btn.add_css_class("active");
            }

            let pid = p.id.clone();
            let stack_clone = self.stack.clone();
            let active_clone = self.active_page_id.clone();
            let bar_clone = self.page_bar_box.clone();

            btn.connect_clicked(move |_| {
                *active_clone.borrow_mut() = pid.clone();
                stack_clone.set_visible_child_name(&pid);

                let mut cur = bar_clone.first_child();
                while let Some(w) = cur {
                    w.remove_css_class("active");
                    cur = w.next_sibling();
                }
            });

            self.page_bar_box.append(&btn);
        }
    }

    pub fn refresh_background_list(&self) {
        while let Some(child) = self.bg_list_box.first_child() {
            self.bg_list_box.remove(&child);
        }

        let active_sessions: Vec<String> = self
            .pages
            .borrow()
            .iter()
            .map(|p| p.session_name.clone())
            .collect();

        // 1. Check tmux sessions
        if let Ok(sessions) = self.tmux.list_sessions() {
            for s in sessions {
                if !active_sessions.contains(&s.name) {
                    let card = GtkBox::new(Orientation::Horizontal, 6);
                    card.add_css_class("session-card");

                    let info_box = GtkBox::new(Orientation::Vertical, 2);
                    info_box.set_hexpand(true);

                    let title = Label::new(Some(&format!("tmux: {}", s.name)));
                    title.add_css_class("session-title");
                    title.set_xalign(0.0);

                    let sub = Label::new(Some(&format!(
                        "clients: {} | windows: {}",
                        s.attached_clients, s.windows_count
                    )));
                    sub.add_css_class("session-sub");
                    sub.set_xalign(0.0);

                    info_box.append(&title);
                    info_box.append(&sub);

                    let btn_reattach = Button::with_label("Reattach");
                    btn_reattach.add_css_class("btn-reattach");

                    let sess_name = s.name.clone();
                    let stack_clone = self.stack.clone();
                    let pages_clone = self.pages.clone();
                    let active_clone = self.active_page_id.clone();
                    let page_bar_clone = self.page_bar_box.clone();
                    let bg_list_clone = self.bg_list_box.clone();
                    let tmux_clone = self.tmux.clone();

                    btn_reattach.connect_clicked(move |_| {
                        // Reattach session as active page
                        let count = pages_clone.borrow().len() + 1;
                        let page_id = format!("page_{}_{}", sess_name, glib::random_int());
                        let title_str = format!("{}: {}", count, sess_name);

                        let terminal = Terminal::new();
                        terminal.set_hexpand(true);
                        terminal.set_vexpand(true);
                        terminal.set_scrollback_lines(50000);

                        let fg = gdk::RGBA::new(0.96, 0.96, 0.96, 1.0);
                        let bg = gdk::RGBA::new(0.094, 0.094, 0.106, 1.0);
                        terminal.set_colors(Some(&fg), Some(&bg), &[]);

                        let tmux_args = [
                            "/usr/bin/tmux",
                            "-L",
                            tmux_clone.socket_name(),
                            "new-session",
                            "-A",
                            "-s",
                            &sess_name,
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

                        let page = TerminalPage {
                            id: page_id.clone(),
                            title: title_str,
                            session_name: sess_name.clone(),
                            terminal: terminal.clone(),
                        };

                        stack_clone.add_named(&terminal, Some(&page_id));
                        pages_clone.borrow_mut().push(page);
                        *active_clone.borrow_mut() = page_id.clone();
                        stack_clone.set_visible_child_name(&page_id);

                        // Trigger UI update
                        let mut cur = page_bar_clone.first_child();
                        while let Some(w) = cur {
                            w.remove_css_class("active");
                            cur = w.next_sibling();
                        }
                        if let Some(card_parent) = bg_list_clone.first_child() {
                            bg_list_clone.remove(&card_parent);
                        }
                    });

                    let btn_kill = Button::with_label("✕");
                    btn_kill.add_css_class("btn-kill-session");
                    let tmux_kill = self.tmux.clone();
                    let card_box = card.clone();
                    let list_box = self.bg_list_box.clone();
                    let s_id = s.id.clone();
                    btn_kill.connect_clicked(move |_| {
                        let _ = tmux_kill.kill_session(&s_id);
                        list_box.remove(&card_box);
                    });

                    card.append(&info_box);
                    card.append(&btn_reattach);
                    card.append(&btn_kill);
                    self.bg_list_box.append(&card);
                }
            }
        }

        // 2. Check systemd background tasks
        if let Ok(tasks) = self.systemd.list_tasks() {
            for t in tasks {
                let card = GtkBox::new(Orientation::Horizontal, 6);
                card.add_css_class("session-card");

                let info_box = GtkBox::new(Orientation::Vertical, 2);
                info_box.set_hexpand(true);

                let title = Label::new(Some(&format!("systemd: {}", t.unit)));
                title.add_css_class("session-title");
                title.set_xalign(0.0);

                let sub = Label::new(Some(&format!("{:?} | {}", t.state, t.command)));
                sub.add_css_class("session-sub");
                sub.set_xalign(0.0);

                info_box.append(&title);
                info_box.append(&sub);

                let btn_kill = Button::with_label("✕");
                btn_kill.add_css_class("btn-kill-session");
                let sys_kill = self.systemd.clone();
                let card_box = card.clone();
                let list_box = self.bg_list_box.clone();
                let u_name = t.unit.clone();
                btn_kill.connect_clicked(move |_| {
                    let _ = sys_kill.stop_task(&u_name);
                    let _ = sys_kill.dismiss_task(&u_name);
                    list_box.remove(&card_box);
                });

                card.append(&info_box);
                card.append(&btn_kill);
                self.bg_list_box.append(&card);
            }
        }
    }
}
