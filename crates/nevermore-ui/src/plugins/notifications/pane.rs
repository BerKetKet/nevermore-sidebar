use super::parser::{DbusNotificationParser, ParsedNotification};
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, GestureClick, Label, Orientation, PolicyType, ScrolledWindow};
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::traits::WindowRaiser;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub type NotificationItem = ParsedNotification;

pub struct NotificationsPane {
    container: GtkBox,
    list_box: GtkBox,
    empty_label: Label,
    dnd_cookie: RefCell<Option<u32>>,
    dbus_conn: Option<gio::DBusConnection>,
    kwin: KWinAdapter,
    count_label: Label,
    items: RefCell<Vec<NotificationItem>>,
    time_labels: RefCell<Vec<(Label, u64)>>,
}

impl NotificationsPane {
    pub fn new(kwin: KWinAdapter) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Header Action Bar
        let header = GtkBox::new(Orientation::Horizontal, 6);
        let title = Label::new(Some("NOTIFICATIONS FEED"));
        title.add_css_class("notif-header-title");
        title.set_hexpand(true);
        title.set_xalign(0.0);

        let count_label = Label::new(Some("0"));
        count_label.add_css_class("notif-count-badge");

        let btn_dnd = Button::with_label("🔔 DND OFF");
        btn_dnd.add_css_class("btn-dnd");
        btn_dnd.set_tooltip_text(Some("Toggle Do Not Disturb"));

        let btn_clear = Button::with_label("Clear All");
        btn_clear.add_css_class("btn-notif-clear");

        header.append(&title);
        header.append(&count_label);
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

        let empty_label = Label::new(Some("No recent notifications"));
        empty_label.add_css_class("notif-empty");
        empty_label.set_vexpand(true);
        empty_label.set_valign(gtk4::Align::Center);
        list_box.append(&empty_label);

        scroll.set_child(Some(&list_box));

        container.append(&header);
        container.append(&scroll);

        let dbus_conn = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok();

        let pane = Rc::new(Self {
            container,
            list_box,
            empty_label,
            dnd_cookie: RefCell::new(None),
            dbus_conn,
            kwin,
            count_label,
            items: RefCell::new(Vec::new()),
            time_labels: RefCell::new(Vec::new()),
        });

        // Connect DND Toggle with persistent session D-Bus connection (P2-2 / N5)
        let pane_dnd = pane.clone();
        btn_dnd.connect_clicked(move |b| {
            pane_dnd.toggle_dnd(b);
        });

        // Connect Clear All
        let pane_clear = pane.clone();
        btn_clear.connect_clicked(move |_| {
            pane_clear.clear_all();
        });

        // Spawn supervised D-Bus Monitor thread (P2-2 / N3 & N4)
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

    pub fn add_notification(&self, item: NotificationItem) {
        if self.items.borrow().is_empty() {
            self.list_box.remove(&self.empty_label);
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

        // Click on notification card raises application window (Spike 08)
        let de = item.desktop_entry.clone();
        let kwin = self.kwin.clone();
        let click_gesture = GestureClick::new();
        click_gesture.connect_released(move |_, _, _, _| {
            let _ = kwin.raise_application(&de);
        });
        card.add_controller(click_gesture);

        // Prepend card so latest notifications are on top
        self.list_box.prepend(&card);
        self.time_labels
            .borrow_mut()
            .push((time_lbl, item.timestamp_unix));
        items.push(item);

        let count = items.len();
        self.count_label.set_label(&count.to_string());
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
            "Just now".to_string()
        } else if elapsed < 3600 {
            format!("{}m ago", elapsed / 60)
        } else if elapsed < 86400 {
            format!("{}h ago", elapsed / 3600)
        } else {
            format!("{}d ago", elapsed / 86400)
        }
    }

    pub fn clear_all(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.items.borrow_mut().clear();
        self.time_labels.borrow_mut().clear();
        self.list_box.append(&self.empty_label);
        self.count_label.set_label("0");
    }

    /// Persistent session D-Bus Inhibit / UnInhibit holding the active connection (N5)
    fn toggle_dnd(&self, btn: &Button) {
        let Some(ref conn) = self.dbus_conn else {
            eprintln!("[nevermore-notif] No active session D-Bus connection available for DND");
            return;
        };

        let mut cookie_slot = self.dnd_cookie.borrow_mut();
        if let Some(cookie) = *cookie_slot {
            // Uninhibit via active D-Bus connection
            let params = (cookie,).to_variant();
            let _ = conn.call_sync(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "UnInhibit",
                Some(&params),
                None,
                gio::DBusCallFlags::NONE,
                -1,
                gio::Cancellable::NONE,
            );

            *cookie_slot = None;
            btn.set_label("🔔 DND OFF");
            btn.remove_css_class("dnd-active");
            println!("[DND] Uninhibited notifications (cookie: {})", cookie);
        } else {
            // Inhibit via active D-Bus connection
            let empty_hints: HashMap<String, glib::Variant> = HashMap::new();
            let params = ("nevermore", "Nevermore DND", empty_hints).to_variant();

            match conn.call_sync(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "Inhibit",
                Some(&params),
                Some(glib::VariantTy::new("(u)").unwrap()),
                gio::DBusCallFlags::NONE,
                -1,
                gio::Cancellable::NONE,
            ) {
                Ok(reply) => {
                    if let Some(c) = reply.child_value(0).get::<u32>() {
                        *cookie_slot = Some(c);
                        btn.set_label("🔕 DND ON");
                        btn.add_css_class("dnd-active");
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

    /// Supervised dbus-monitor with kernel PR_SET_PDEATHSIG + auto-restart backoff (N3 & N4)
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
