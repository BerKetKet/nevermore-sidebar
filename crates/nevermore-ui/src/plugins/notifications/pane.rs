use super::parser::{DbusNotificationParser, ParsedNotification};
use crate::icons;
use gtk4::accessible::Property;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DrawingArea, GestureClick, Label, Orientation, PolicyType,
    ScrolledWindow,
};
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::theme::{contrast_text_color_for_hex, ThemePalette};
use nevermore_core::traits::WindowRaiser;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::rc::{Rc, Weak};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub type NotificationItem = ParsedNotification;

pub struct NotificationsPane {
    container: GtkBox,
    list_box: GtkBox,
    empty_box: GtkBox,
    empty_img: DrawingArea,
    btn_dnd: Button,
    btn_clear: Button,
    dnd_cookie: RefCell<Option<u32>>,
    dbus_conn: Option<gio::DBusConnection>,
    kwin: KWinAdapter,
    items: RefCell<Vec<NotificationItem>>,
    time_labels: RefCell<Vec<(Label, u64)>>,
    current_theme: RefCell<ThemePalette>,
    self_weak: RefCell<Option<Weak<NotificationsPane>>>,
}

impl NotificationsPane {
    pub fn new(kwin: KWinAdapter, theme: &ThemePalette) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Header Action Bar: Spacer + DND icon button + Clear icon button
        let header = GtkBox::new(Orientation::Horizontal, 6);

        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let btn_dnd = Button::new();
        btn_dnd.add_css_class("btn-dnd");
        btn_dnd.set_tooltip_text(Some("Toggle Do Not Disturb"));
        btn_dnd.update_property(&[Property::Label("Toggle Do Not Disturb")]);
        icons::set_button_icon(&btn_dnd, "bell", &theme.text_secondary, 16);

        let btn_clear = Button::new();
        btn_clear.add_css_class("btn-notif-clear");
        btn_clear.set_tooltip_text(Some("Clear All"));
        btn_clear.update_property(&[Property::Label("Clear All")]);
        icons::set_button_icon(&btn_clear, "trash-2", &theme.text_secondary, 16);

        header.append(&spacer);
        header.append(&btn_dnd);
        header.append(&btn_clear);

        // Scrollable Notifications List
        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .build();
        scroll.add_css_class("notif-scroll");

        let list_box = GtkBox::new(Orientation::Vertical, 6);
        list_box.set_hexpand(true);

        // Text-free empty state: Centered dim 32px Lucide inbox icon
        let empty_box = GtkBox::new(Orientation::Vertical, 0);
        empty_box.set_vexpand(true);
        empty_box.set_valign(gtk4::Align::Center);
        empty_box.set_halign(gtk4::Align::Center);
        empty_box.add_css_class("notif-empty");

        let empty_img = icons::create_icon_widget("inbox", &theme.text_secondary, 32);
        empty_box.append(&empty_img);
        list_box.append(&empty_box);

        scroll.set_child(Some(&list_box));

        container.append(&header);
        container.append(&scroll);

        let dbus_conn = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok();

        let pane = Rc::new(Self {
            container,
            list_box,
            empty_box,
            empty_img,
            btn_dnd: btn_dnd.clone(),
            btn_clear: btn_clear.clone(),
            dnd_cookie: RefCell::new(None),
            dbus_conn,
            kwin,
            items: RefCell::new(Vec::new()),
            time_labels: RefCell::new(Vec::new()),
            current_theme: RefCell::new(theme.clone()),
            self_weak: RefCell::new(None),
        });

        *pane.self_weak.borrow_mut() = Some(Rc::downgrade(&pane));

        // Connect DND Toggle
        let pane_dnd = pane.clone();
        btn_dnd.connect_clicked(move |_| {
            pane_dnd.toggle_dnd();
        });

        // Connect Clear All
        let pane_clear = pane.clone();
        btn_clear.connect_clicked(move |_| {
            pane_clear.clear_all();
        });

        // Spawn supervised D-Bus Monitor thread
        let (tx, rx) = async_channel::unbounded::<NotificationItem>();
        Self::spawn_dbus_monitor(tx);

        // Handle incoming notifications on the GTK main loop
        let pane_recv = pane.clone();
        glib::spawn_future_local(async move {
            while let Ok(item) = rx.recv().await {
                pane_recv.add_notification(item);
            }
        });

        pane
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn apply_theme(&self, theme: &ThemePalette) {
        *self.current_theme.borrow_mut() = theme.clone();

        self.empty_img.queue_draw();

        icons::set_button_icon(&self.btn_clear, "trash-2", &theme.text_secondary, 16);

        if self.dnd_cookie.borrow().is_some() {
            let accent_fg = contrast_text_color_for_hex(&theme.accent_color);
            icons::set_button_icon(&self.btn_dnd, "bell-off", accent_fg, 16);
        } else {
            icons::set_button_icon(&self.btn_dnd, "bell", &theme.text_secondary, 16);
        }
    }

    pub fn add_notification(&self, item: NotificationItem) {
        if self.items.borrow().is_empty() {
            self.list_box.remove(&self.empty_box);
        }

        // Cap at 100 items: evict oldest if at capacity
        let mut items = self.items.borrow_mut();
        if items.len() >= 100 {
            items.remove(0);
            if let Some(last_child) = self.list_box.last_child() {
                self.list_box.remove(&last_child);
            }
            let mut time_labels = self.time_labels.borrow_mut();
            if !time_labels.is_empty() {
                time_labels.remove(0);
            }
        }

        let card = GtkBox::new(Orientation::Vertical, 4);
        card.add_css_class("notif-card");

        let header_row = GtkBox::new(Orientation::Horizontal, 6);
        let app_lbl = Label::new(Some(&item.app_name));
        app_lbl.add_css_class("notif-app-name");
        app_lbl.set_xalign(0.0);
        app_lbl.set_hexpand(true);

        let time_str = Self::format_relative_time(item.timestamp_unix);
        let time_lbl = Label::new(Some(&time_str));
        time_lbl.add_css_class("notif-time");
        time_lbl.set_xalign(1.0);

        header_row.append(&app_lbl);
        header_row.append(&time_lbl);

        let summary_lbl = Label::new(Some(&item.summary));
        summary_lbl.add_css_class("notif-summary");
        summary_lbl.set_xalign(0.0);
        summary_lbl.set_wrap(true);

        card.append(&header_row);
        card.append(&summary_lbl);

        if !item.body.is_empty() {
            let body_lbl = Label::new(Some(&item.body));
            body_lbl.add_css_class("notif-body");
            body_lbl.set_xalign(0.0);
            body_lbl.set_wrap(true);
            card.append(&body_lbl);
        }

        // Click on notification card raises application window non-blocking (A2)
        let de = item.desktop_entry.clone();
        let kwin = self.kwin.clone();
        let click_gesture = GestureClick::new();
        click_gesture.connect_released(move |_, _, _, _| {
            let de_clone = de.clone();
            let kwin_clone = kwin.clone();
            glib::spawn_future_local(async move {
                let _ = gio::spawn_blocking(move || {
                    let _ = kwin_clone.raise_application(&de_clone);
                })
                .await;
            });
        });
        card.add_controller(click_gesture);

        // Prepend card so latest notifications are on top
        self.list_box.prepend(&card);
        self.time_labels
            .borrow_mut()
            .push((time_lbl, item.timestamp_unix));
        items.push(item);
    }

    pub fn update_relative_times(&self) {
        for (lbl, ts) in self.time_labels.borrow().iter() {
            lbl.set_label(&Self::format_relative_time(*ts));
        }
    }

    fn format_relative_time(timestamp_unix: u64) -> String {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now_unix.saturating_sub(timestamp_unix);
        if elapsed < 60 {
            "now".to_string()
        } else if elapsed < 3600 {
            format!("{}m", elapsed / 60)
        } else if elapsed < 86400 {
            format!("{}h", elapsed / 3600)
        } else {
            format!("{}d", elapsed / 86400)
        }
    }

    pub fn clear_all(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.items.borrow_mut().clear();
        self.time_labels.borrow_mut().clear();
        self.list_box.append(&self.empty_box);
    }

    /// Asynchronous non-blocking DND toggle with 2000ms timeout (A2)
    pub fn toggle_dnd(self: &Rc<Self>) {
        let Some(ref conn) = self.dbus_conn else {
            eprintln!("[nevermore-notif] No active session D-Bus connection available for DND");
            return;
        };

        let conn = conn.clone();
        let weak = self.self_weak.borrow().clone();
        let is_currently_inhibited = self.dnd_cookie.borrow().is_some();

        if is_currently_inhibited {
            let cookie = self.dnd_cookie.borrow().unwrap();
            let params = (cookie,).to_variant();

            glib::spawn_future_local(async move {
                let res = conn
                    .call_future(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        "UnInhibit",
                        Some(&params),
                        None,
                        gio::DBusCallFlags::NONE,
                        2000,
                    )
                    .await;

                if let Some(w) = weak {
                    if let Some(pane) = w.upgrade() {
                        match res {
                            Ok(_) => {
                                *pane.dnd_cookie.borrow_mut() = None;
                                pane.btn_dnd.remove_css_class("dnd-active");
                                let theme = pane.current_theme.borrow();
                                icons::set_button_icon(
                                    &pane.btn_dnd,
                                    "bell",
                                    &theme.text_secondary,
                                    16,
                                );
                                println!("[DND] Uninhibited notifications (cookie: {})", cookie);
                            }
                            Err(e) => {
                                eprintln!("[nevermore-notif] Failed to UnInhibit: {}", e);
                            }
                        }
                    }
                }
            });
        } else {
            let empty_hints: HashMap<String, glib::Variant> = HashMap::new();
            let params = ("nevermore", "Nevermore DND", empty_hints).to_variant();

            glib::spawn_future_local(async move {
                let res = conn
                    .call_future(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        "Inhibit",
                        Some(&params),
                        Some(glib::VariantTy::new("(u)").unwrap()),
                        gio::DBusCallFlags::NONE,
                        2000,
                    )
                    .await;

                if let Some(w) = weak {
                    if let Some(pane) = w.upgrade() {
                        match res {
                            Ok(reply) => {
                                if let Some(c) = reply.child_value(0).get::<u32>() {
                                    *pane.dnd_cookie.borrow_mut() = Some(c);
                                    pane.btn_dnd.add_css_class("dnd-active");
                                    let theme = pane.current_theme.borrow();
                                    let accent_fg =
                                        contrast_text_color_for_hex(&theme.accent_color);
                                    icons::set_button_icon(
                                        &pane.btn_dnd,
                                        "bell-off",
                                        accent_fg,
                                        16,
                                    );
                                    println!(
                                        "[DND] Inhibited notifications on active bus (cookie: {})",
                                        c
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("[nevermore-notif] Failed to call D-Bus Inhibit: {}", e);
                            }
                        }
                    }
                }
            });
        }
    }

    /// Asynchronously check if D-Bus Notifications daemon still has inhibition active (A5)
    pub fn refresh_dnd_state(self: &Rc<Self>) {
        let Some(ref conn) = self.dbus_conn else {
            return;
        };

        if self.dnd_cookie.borrow().is_none() {
            return;
        }

        let conn = conn.clone();
        let weak = self.self_weak.borrow().clone();

        glib::spawn_future_local(async move {
            let params = ("org.freedesktop.Notifications", "Inhibited").to_variant();
            let res = conn
                .call_future(
                    Some("org.freedesktop.Notifications"),
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    Some(&params),
                    Some(glib::VariantTy::new("(v)").unwrap()),
                    gio::DBusCallFlags::NONE,
                    1000,
                )
                .await;

            if let Some(w) = weak {
                if let Some(pane) = w.upgrade() {
                    match res {
                        Ok(val) => {
                            let is_inhibited = val
                                .child_value(0)
                                .child_value(0)
                                .get::<bool>()
                                .unwrap_or(false);
                            if !is_inhibited && pane.dnd_cookie.borrow().is_some() {
                                *pane.dnd_cookie.borrow_mut() = None;
                                pane.btn_dnd.remove_css_class("dnd-active");
                                let theme = pane.current_theme.borrow();
                                icons::set_button_icon(
                                    &pane.btn_dnd,
                                    "bell",
                                    &theme.text_secondary,
                                    16,
                                );
                            }
                        }
                        Err(_) => {
                            // Daemon might have restarted or unreachable
                            *pane.dnd_cookie.borrow_mut() = None;
                            pane.btn_dnd.remove_css_class("dnd-active");
                            let theme = pane.current_theme.borrow();
                            icons::set_button_icon(
                                &pane.btn_dnd,
                                "bell",
                                &theme.text_secondary,
                                16,
                            );
                        }
                    }
                }
            }
        });
    }

    /// Supervised dbus-monitor with kernel PR_SET_PDEATHSIG + auto-restart backoff
    fn spawn_dbus_monitor(tx: async_channel::Sender<NotificationItem>) {
        thread::spawn(move || {
            let parent_pid = std::process::id() as libc::pid_t;

            loop {
                let mut cmd = Command::new("dbus-monitor");
                cmd.arg("--session")
                    .arg("type='method_call',interface='org.freedesktop.Notifications',member='Notify'")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null());

                // Kernel orphan guard: PR_SET_PDEATHSIG + fork-race check
                unsafe {
                    cmd.pre_exec(move || {
                        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                        if libc::getppid() != parent_pid {
                            libc::_exit(1);
                        }
                        Ok(())
                    });
                }

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[nevermore-notif] Failed to spawn dbus-monitor: {}", e);
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };

                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    let mut parser = DbusNotificationParser::new();

                    while let Some(Ok(line)) = lines.next() {
                        if let Some(item) = parser.feed_line(&line) {
                            let _ = tx.send_blocking(item);
                        }
                    }
                }

                let _ = child.wait();
                eprintln!("[nevermore-notif] dbus-monitor exited; restarting in 2 seconds...");
                thread::sleep(Duration::from_secs(2));
            }
        });
    }
}
