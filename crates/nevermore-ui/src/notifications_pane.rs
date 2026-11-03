use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, GestureClick, Label, Orientation, PolicyType, ScrolledWindow};
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::traits::WindowRaiser;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct NotificationItem {
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub desktop_entry: String,
    pub timestamp_unix: u64,
}

pub struct NotificationsPane {
    container: GtkBox,
    list_box: GtkBox,
    empty_label: Label,
    dnd_cookie: RefCell<Option<u32>>,
    kwin: KWinAdapter,
    count_label: Label,
    items: RefCell<Vec<NotificationItem>>,
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

        let pane = Rc::new(Self {
            container,
            list_box,
            empty_label,
            dnd_cookie: RefCell::new(None),
            kwin,
            count_label,
            items: RefCell::new(Vec::new()),
        });

        // Connect DND Toggle (Spike 09)
        let pane_dnd = pane.clone();
        btn_dnd.connect_clicked(move |b| {
            pane_dnd.toggle_dnd(b);
        });

        // Connect Clear All
        let pane_clear = pane.clone();
        btn_clear.connect_clicked(move |_| {
            pane_clear.clear_all();
        });

        // Spawn D-Bus Listen-Only Monitor thread (Spike 07)
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

        let card = GtkBox::new(Orientation::Vertical, 4);
        card.add_css_class("notif-card");

        let header_row = GtkBox::new(Orientation::Horizontal, 6);
        let app_lbl = Label::new(Some(&item.app_name));
        app_lbl.add_css_class("notif-app-name");
        app_lbl.set_xalign(0.0);
        app_lbl.set_hexpand(true);

        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now_unix.saturating_sub(item.timestamp_unix);
        let time_str = if elapsed < 60 {
            "Just now".to_string()
        } else {
            format!("{}m ago", elapsed / 60)
        };
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
        self.items.borrow_mut().push(item);

        let count = self.items.borrow().len();
        self.count_label.set_label(&count.to_string());
    }

    pub fn clear_all(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.items.borrow_mut().clear();
        self.list_box.append(&self.empty_label);
        self.count_label.set_label("0");
    }

    fn toggle_dnd(&self, btn: &Button) {
        let mut cookie_slot = self.dnd_cookie.borrow_mut();
        if let Some(cookie) = *cookie_slot {
            // UnInhibit
            let _ = Command::new("busctl")
                .arg("--user")
                .arg("call")
                .arg("org.freedesktop.Notifications")
                .arg("/org/freedesktop/Notifications")
                .arg("org.freedesktop.Notifications")
                .arg("UnInhibit")
                .arg("u")
                .arg(cookie.to_string())
                .status();
            *cookie_slot = None;
            btn.set_label("🔔 DND OFF");
            btn.remove_css_class("dnd-active");
        } else {
            // Inhibit
            let out = Command::new("busctl")
                .arg("--user")
                .arg("call")
                .arg("org.freedesktop.Notifications")
                .arg("/org/freedesktop/Notifications")
                .arg("org.freedesktop.Notifications")
                .arg("Inhibit")
                .arg("ssa{sv}")
                .arg("nevermore")
                .arg("Nevermore DND")
                .arg("0")
                .output();

            if let Ok(o) = out {
                if o.status.success() {
                    let s = String::from_utf8_lossy(&o.stdout);
                    if let Some(c_str) = s.split_whitespace().nth(1) {
                        if let Ok(c) = c_str.parse::<u32>() {
                            *cookie_slot = Some(c);
                            btn.set_label("🔕 DND ON");
                            btn.add_css_class("dnd-active");
                        }
                    }
                }
            }
        }
    }

    fn spawn_dbus_monitor(tx: async_channel::Sender<NotificationItem>) {
        thread::spawn(move || {
            let mut cmd = Command::new("dbus-monitor");
            cmd.arg("--session")
                .arg("type='method_call',interface='org.freedesktop.Notifications',member='Notify'")
                .stdout(Stdio::piped())
                .stderr(Stdio::null());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(_) => return,
            };

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut current_strings: Vec<String> = Vec::new();

                while let Some(Ok(line)) = lines.next() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("method call") {
                        // Deliver previously collected notification if complete
                        if current_strings.len() >= 4 {
                            let app_name = current_strings[0].clone();
                            let summary = current_strings[2].clone();
                            let body = current_strings[3].clone();
                            let desktop_entry = app_name.to_lowercase().replace(' ', "-");
                            let ts = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            let item = NotificationItem {
                                app_name,
                                summary,
                                body,
                                desktop_entry,
                                timestamp_unix: ts,
                            };
                            let _ = tx.send_blocking(item);
                        }
                        current_strings.clear();
                    } else if trimmed.starts_with("string \"") && trimmed.ends_with('"') {
                        let content = &trimmed[8..trimmed.len() - 1];
                        current_strings.push(content.to_string());
                    }
                }
            }
        });
    }
}
