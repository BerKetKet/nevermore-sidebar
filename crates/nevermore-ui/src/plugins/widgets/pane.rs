use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ProgressBar, Scale};
use std::cell::RefCell;
use std::fs;
use std::process::Command;
use std::rc::Rc;

pub struct WidgetsPane {
    container: GtkBox,
    time_lbl: Label,
    date_lbl: Label,
    cpu_bar: ProgressBar,
    cpu_label: Label,
    ram_bar: ProgressBar,
    ram_label: Label,
    media_title: Label,
    media_artist: Label,
    last_cpu: RefCell<Option<(u64, u64)>>,
    timer_source: RefCell<Option<glib::SourceId>>,
}

impl WidgetsPane {
    pub fn new() -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // 1. System Clock / Date Card
        let clock_card = GtkBox::new(Orientation::Vertical, 2);
        clock_card.add_css_class("widget-card");
        let time_lbl = Label::new(None);
        time_lbl.add_css_class("clock-time");
        time_lbl.set_xalign(0.0);
        let date_lbl = Label::new(None);
        date_lbl.add_css_class("clock-date");
        date_lbl.set_xalign(0.0);
        clock_card.append(&time_lbl);
        clock_card.append(&date_lbl);

        // 2. Resource Utilization (CPU & RAM)
        let stats_card = GtkBox::new(Orientation::Vertical, 8);
        stats_card.add_css_class("widget-card");

        let stats_header = Label::new(Some("SYSTEM RESOURCES"));
        stats_header.add_css_class("widget-header");
        stats_header.set_xalign(0.0);
        stats_card.append(&stats_header);

        // CPU
        let cpu_box = GtkBox::new(Orientation::Horizontal, 8);
        let cpu_title = Label::new(Some("CPU"));
        cpu_title.set_size_request(35, -1);
        cpu_title.set_xalign(0.0);
        let cpu_bar = ProgressBar::new();
        cpu_bar.set_hexpand(true);
        let cpu_label = Label::new(Some("0%"));
        cpu_label.set_size_request(45, -1);
        cpu_label.set_xalign(1.0);
        cpu_box.append(&cpu_title);
        cpu_box.append(&cpu_bar);
        cpu_box.append(&cpu_label);
        stats_card.append(&cpu_box);

        // RAM
        let ram_box = GtkBox::new(Orientation::Horizontal, 8);
        let ram_title = Label::new(Some("RAM"));
        ram_title.set_size_request(35, -1);
        ram_title.set_xalign(0.0);
        let ram_bar = ProgressBar::new();
        ram_bar.set_hexpand(true);
        let ram_label = Label::new(Some("0%"));
        ram_label.set_size_request(45, -1);
        ram_label.set_xalign(1.0);
        ram_box.append(&ram_title);
        ram_box.append(&ram_bar);
        ram_box.append(&ram_label);
        stats_card.append(&ram_box);

        // 3. Audio & Volume (PipeWire via wpctl)
        let audio_card = GtkBox::new(Orientation::Vertical, 6);
        audio_card.add_css_class("widget-card");

        let audio_header = Label::new(Some("AUDIO OUTPUT"));
        audio_header.add_css_class("widget-header");
        audio_header.set_xalign(0.0);
        audio_card.append(&audio_header);

        let audio_row = GtkBox::new(Orientation::Horizontal, 8);
        let btn_mute = Button::with_label("🔊");
        btn_mute.add_css_class("btn-mute");
        btn_mute.set_tooltip_text(Some("Toggle Mute"));

        let vol_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 5.0);
        vol_scale.set_hexpand(true);
        vol_scale.set_draw_value(false);

        audio_row.append(&btn_mute);
        audio_row.append(&vol_scale);
        audio_card.append(&audio_row);

        // 4. MPRIS2 Media Controller Card
        let media_card = GtkBox::new(Orientation::Vertical, 6);
        media_card.add_css_class("widget-card");

        let media_header = Label::new(Some("MEDIA CONTROLLER"));
        media_header.add_css_class("widget-header");
        media_header.set_xalign(0.0);
        media_card.append(&media_header);

        let media_title = Label::new(Some("No active player"));
        media_title.add_css_class("media-title");
        media_title.set_xalign(0.0);
        media_title.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let media_artist = Label::new(Some("Idle"));
        media_artist.add_css_class("media-artist");
        media_artist.set_xalign(0.0);
        media_artist.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let media_controls = GtkBox::new(Orientation::Horizontal, 8);
        media_controls.set_halign(gtk4::Align::Center);

        let btn_prev = Button::with_label("⏮");
        let btn_play_pause = Button::with_label("⏯");
        let btn_next = Button::with_label("⏭");

        btn_prev.add_css_class("btn-media");
        btn_play_pause.add_css_class("btn-media");
        btn_next.add_css_class("btn-media");

        media_controls.append(&btn_prev);
        media_controls.append(&btn_play_pause);
        media_controls.append(&btn_next);

        media_card.append(&media_title);
        media_card.append(&media_artist);
        media_card.append(&media_controls);

        container.append(&clock_card);
        container.append(&stats_card);
        container.append(&audio_card);
        container.append(&media_card);

        let pane = Rc::new(Self {
            container,
            time_lbl,
            date_lbl,
            cpu_bar,
            cpu_label,
            ram_bar,
            ram_label,
            media_title,
            media_artist,
            last_cpu: RefCell::new(None),
            timer_source: RefCell::new(None),
        });

        // Initialize Volume slider from wpctl
        if let Ok(vol) = Self::get_wpctl_volume() {
            vol_scale.set_value(vol * 100.0);
        }

        // Connect volume slider
        vol_scale.connect_value_changed(|scale| {
            let val = scale.value() / 100.0;
            let _ = Command::new("wpctl")
                .arg("set-volume")
                .arg("@DEFAULT_AUDIO_SINK@")
                .arg(format!("{:.2}", val))
                .status();
        });

        // Connect mute button
        let btn_mute_clone = btn_mute.clone();
        btn_mute.connect_clicked(move |_| {
            let _ = Command::new("wpctl")
                .arg("set-mute")
                .arg("@DEFAULT_AUDIO_SINK@")
                .arg("toggle")
                .status();
            if let Ok(muted) = Self::is_wpctl_muted() {
                if muted {
                    btn_mute_clone.set_label("🔇");
                } else {
                    btn_mute_clone.set_label("🔊");
                }
            }
        });

        // Connect MPRIS controls
        btn_prev.connect_clicked(|_| {
            let _ = Self::send_mpris_cmd("Previous");
        });
        btn_play_pause.connect_clicked(|_| {
            let _ = Self::send_mpris_cmd("PlayPause");
        });
        btn_next.connect_clicked(|_| {
            let _ = Self::send_mpris_cmd("Next");
        });

        pane
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Triggers its 2s /proc timer strictly on show (Option 1 Lifecycle)
    pub fn on_show(self: &Rc<Self>) {
        if self.timer_source.borrow().is_none() {
            // Immediate update
            self.update_clock();
            self.update_metrics();
            self.update_media();

            let weak = Rc::downgrade(self);
            let source_id = glib::timeout_add_seconds_local(2, move || {
                if let Some(p) = weak.upgrade() {
                    p.update_clock();
                    p.update_metrics();
                    p.update_media();
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            });
            *self.timer_source.borrow_mut() = Some(source_id);
        }
    }

    /// Stops the 2s /proc timer completely when hidden or tab inactive
    pub fn on_hide(&self) {
        if let Some(source_id) = self.timer_source.borrow_mut().take() {
            source_id.remove();
        }
    }

    fn update_clock(&self) {
        if let Ok(now) = glib::DateTime::now_local() {
            if let Ok(time_str) = now.format("%H:%M:%S") {
                self.time_lbl.set_label(&time_str);
            }
            if let Ok(date_str) = now.format("%A, %B %d, %Y") {
                self.date_lbl.set_label(&date_str);
            }
        }
    }

    fn update_metrics(&self) {
        // 1. CPU Usage from /proc/stat
        if let Ok(stat) = fs::read_to_string("/proc/stat") {
            if let Some(first_line) = stat.lines().next() {
                let parts: Vec<u64> = first_line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if parts.len() >= 4 {
                    let idle = parts[3];
                    let total: u64 = parts.iter().sum();
                    if let Some((prev_total, prev_idle)) = *self.last_cpu.borrow() {
                        let total_delta = total.saturating_sub(prev_total);
                        let idle_delta = idle.saturating_sub(prev_idle);
                        if total_delta > 0 {
                            let usage = (total_delta.saturating_sub(idle_delta)) as f64
                                / total_delta as f64;
                            let pct = (usage * 100.0).clamp(0.0, 100.0);
                            self.cpu_bar.set_fraction(pct / 100.0);
                            self.cpu_label.set_label(&format!("{:.0}%", pct));
                        }
                    }
                    *self.last_cpu.borrow_mut() = Some((total, idle));
                }
            }
        }

        // 2. RAM Usage from /proc/meminfo
        if let Ok(mem) = fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0u64;
            let mut avail_kb = 0u64;
            for line in mem.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    avail_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
            }
            if total_kb > 0 {
                let used_kb = total_kb.saturating_sub(avail_kb);
                let pct = (used_kb as f64 / total_kb as f64) * 100.0;
                self.ram_bar.set_fraction(pct / 100.0);
                self.ram_label.set_label(&format!("{:.0}%", pct));
            }
        }
    }

    fn update_media(&self) {
        if let Ok(output) = Command::new("playerctl").arg("metadata").output() {
            if output.status.success() {
                let out = String::from_utf8_lossy(&output.stdout);
                let mut title = String::new();
                let mut artist = String::new();
                for line in out.lines() {
                    if line.contains("xesam:title") {
                        title = line
                            .split_whitespace()
                            .skip(2)
                            .collect::<Vec<_>>()
                            .join(" ");
                    } else if line.contains("xesam:artist") {
                        artist = line
                            .split_whitespace()
                            .skip(2)
                            .collect::<Vec<_>>()
                            .join(" ");
                    }
                }
                if !title.is_empty() {
                    self.media_title.set_label(&title);
                    self.media_artist.set_label(if artist.is_empty() {
                        "Unknown Artist"
                    } else {
                        &artist
                    });
                    return;
                }
            }
        }
        self.media_title.set_label("No active player");
        self.media_artist.set_label("Idle");
    }

    fn get_wpctl_volume() -> Result<f64, ()> {
        let out = Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()
            .map_err(|_| ())?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(v_str) = s.split_whitespace().nth(1) {
                if let Ok(v) = v_str.parse::<f64>() {
                    return Ok(v);
                }
            }
        }
        Err(())
    }

    fn is_wpctl_muted() -> Result<bool, ()> {
        let out = Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()
            .map_err(|_| ())?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            return Ok(s.contains("[MUTED]"));
        }
        Err(())
    }

    fn send_mpris_cmd(cmd: &str) -> Result<(), ()> {
        Command::new("playerctl")
            .arg(cmd.to_lowercase())
            .status()
            .map(|_| ())
            .map_err(|_| ())
    }
}
