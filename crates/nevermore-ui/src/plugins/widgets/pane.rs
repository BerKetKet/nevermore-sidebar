use crate::icons;
use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Label, Orientation, ProgressBar, Scale, Stack};
use nevermore_core::theme::ThemePalette;
use std::cell::RefCell;
use std::fs;
use std::process::Command;
use std::rc::Rc;
use std::thread;

pub struct WidgetsPane {
    container: GtkBox,
    time_lbl: Label,
    date_lbl: Label,
    cpu_bar: ProgressBar,
    cpu_label: Label,
    ram_bar: ProgressBar,
    ram_label: Label,
    #[allow(dead_code)]
    btn_mute: Button,
    #[allow(dead_code)]
    media_stack: Stack,
    #[allow(dead_code)]
    media_title: Label,
    #[allow(dead_code)]
    media_artist: Label,
    btn_prev: Button,
    btn_play_pause: Button,
    btn_next: Button,
    cpu_icon: DrawingArea,
    ram_icon: DrawingArea,
    idle_music_icon: DrawingArea,
    palette: RefCell<ThemePalette>,
    last_cpu: RefCell<Option<(u64, u64)>>,
    timer_source: RefCell<Option<glib::SourceId>>,
    vol_debounce: RefCell<Option<glib::SourceId>>,
    media_tx: async_channel::Sender<(String, String)>,
    audio_tx: async_channel::Sender<(Option<f64>, Option<bool>)>,
}

impl WidgetsPane {
    pub fn new(theme: &ThemePalette) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 10);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_hexpand(true);

        // 1. Clock & Date Card
        let clock_card = GtkBox::new(Orientation::Vertical, 2);
        clock_card.add_css_class("widget-card");

        let time_lbl = Label::new(None);
        time_lbl.add_css_class("clock-time");
        time_lbl.set_xalign(0.5);

        let date_lbl = Label::new(None);
        date_lbl.add_css_class("clock-date");
        date_lbl.set_xalign(0.5);

        clock_card.append(&time_lbl);
        clock_card.append(&date_lbl);

        // 2. System Resources Card (zero section text headers)
        let stats_card = GtkBox::new(Orientation::Vertical, 8);
        stats_card.add_css_class("widget-card");

        // CPU row: Lucide cpu icon + bar + %
        let cpu_box = GtkBox::new(Orientation::Horizontal, 8);
        let cpu_icon = icons::create_icon_widget("cpu", &theme.text_secondary, 16);
        let cpu_bar = ProgressBar::new();
        cpu_bar.set_hexpand(true);
        let cpu_label = Label::new(Some("0%"));
        cpu_label.set_size_request(45, -1);
        cpu_label.set_xalign(1.0);
        cpu_box.append(&cpu_icon);
        cpu_box.append(&cpu_bar);
        cpu_box.append(&cpu_label);
        stats_card.append(&cpu_box);

        // RAM row: Lucide memory-stick icon + bar + %
        let ram_box = GtkBox::new(Orientation::Horizontal, 8);
        let ram_icon = icons::create_icon_widget("memory-stick", &theme.text_secondary, 16);
        let ram_bar = ProgressBar::new();
        ram_bar.set_hexpand(true);
        let ram_label = Label::new(Some("0%"));
        ram_label.set_size_request(45, -1);
        ram_label.set_xalign(1.0);
        ram_box.append(&ram_icon);
        ram_box.append(&ram_bar);
        ram_box.append(&ram_label);
        stats_card.append(&ram_box);

        // 3. Audio & Volume (PipeWire via wpctl, zero section text headers)
        let audio_card = GtkBox::new(Orientation::Vertical, 6);
        audio_card.add_css_class("widget-card");

        let audio_row = GtkBox::new(Orientation::Horizontal, 8);
        let btn_mute = Button::new();
        btn_mute.add_css_class("btn-mute");
        btn_mute.set_tooltip_text(Some("Toggle Mute"));
        btn_mute.update_property(&[Property::Label("Toggle Mute")]);
        icons::set_button_icon(&btn_mute, "volume-2", &theme.text_secondary, 16);

        let vol_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 5.0);
        vol_scale.set_hexpand(true);
        vol_scale.set_draw_value(false);

        audio_row.append(&btn_mute);
        audio_row.append(&vol_scale);
        audio_card.append(&audio_row);

        // 4. MPRIS2 Media Controller Card (zero section text headers)
        let media_card = GtkBox::new(Orientation::Vertical, 6);
        media_card.add_css_class("widget-card");

        let media_stack = Stack::new();

        // Idle view: centered dim music icon (24px, text_secondary), ZERO text!
        let idle_box = GtkBox::new(Orientation::Vertical, 0);
        idle_box.set_halign(gtk4::Align::Center);
        idle_box.set_valign(gtk4::Align::Center);
        let idle_music_icon = icons::create_icon_widget("music", &theme.text_secondary, 24);
        idle_music_icon.set_opacity(0.5);
        idle_box.append(&idle_music_icon);
        media_stack.add_named(&idle_box, Some("idle"));

        // Playing view: track title & artist (data) + playback controls
        let playing_box = GtkBox::new(Orientation::Vertical, 4);
        let media_title = Label::new(None);
        media_title.add_css_class("media-title");
        media_title.set_xalign(0.0);
        media_title.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let media_artist = Label::new(None);
        media_artist.add_css_class("media-artist");
        media_artist.set_xalign(0.0);
        media_artist.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let media_controls = GtkBox::new(Orientation::Horizontal, 8);
        media_controls.set_halign(gtk4::Align::Center);

        let btn_prev = Button::new();
        btn_prev.add_css_class("btn-media");
        btn_prev.set_tooltip_text(Some("Previous Track"));
        btn_prev.update_property(&[Property::Label("Previous Track")]);
        icons::set_button_icon(&btn_prev, "skip-back", &theme.text_secondary, 16);

        let btn_play_pause = Button::new();
        btn_play_pause.add_css_class("btn-media");
        btn_play_pause.set_tooltip_text(Some("Play / Pause"));
        btn_play_pause.update_property(&[Property::Label("Play / Pause")]);
        icons::set_button_icon(&btn_play_pause, "play", &theme.text_secondary, 16);

        let btn_next = Button::new();
        btn_next.add_css_class("btn-media");
        btn_next.set_tooltip_text(Some("Next Track"));
        btn_next.update_property(&[Property::Label("Next Track")]);
        icons::set_button_icon(&btn_next, "skip-forward", &theme.text_secondary, 16);

        media_controls.append(&btn_prev);
        media_controls.append(&btn_play_pause);
        media_controls.append(&btn_next);

        playing_box.append(&media_title);
        playing_box.append(&media_artist);
        playing_box.append(&media_controls);
        media_stack.add_named(&playing_box, Some("playing"));
        media_stack.set_visible_child_name("idle");

        media_card.append(&media_stack);

        container.append(&clock_card);
        container.append(&stats_card);
        container.append(&audio_card);
        container.append(&media_card);

        // Background channels for async offloading
        let (media_tx, media_rx) = async_channel::unbounded::<(String, String)>();
        let (audio_tx, audio_rx) = async_channel::unbounded::<(Option<f64>, Option<bool>)>();

        // Handle media updates on main GTK loop
        let m_stack = media_stack.clone();
        let m_title = media_title.clone();
        let m_artist = media_artist.clone();
        glib::spawn_future_local(async move {
            while let Ok((title, artist)) = media_rx.recv().await {
                if !title.is_empty() {
                    m_title.set_label(&title);
                    m_artist.set_label(if artist.is_empty() { "" } else { &artist });
                    m_stack.set_visible_child_name("playing");
                } else {
                    m_stack.set_visible_child_name("idle");
                }
            }
        });

        // Handle audio updates on main GTK loop
        let a_scale = vol_scale.clone();
        let a_mute = btn_mute.clone();
        let palette_for_audio = Rc::new(RefCell::new(theme.clone()));
        let p_audio = palette_for_audio.clone();
        glib::spawn_future_local(async move {
            while let Ok((vol_opt, mute_opt)) = audio_rx.recv().await {
                if let Some(v) = vol_opt {
                    a_scale.set_value(v * 100.0);
                }
                if let Some(m) = mute_opt {
                    let pal = p_audio.borrow();
                    let icon_name = if m { "volume-x" } else { "volume-2" };
                    icons::set_button_icon(&a_mute, icon_name, &pal.text_secondary, 16);
                }
            }
        });

        let pane = Rc::new(Self {
            container,
            time_lbl,
            date_lbl,
            cpu_bar,
            cpu_label,
            ram_bar,
            ram_label,
            btn_mute: btn_mute.clone(),
            media_stack,
            media_title,
            media_artist,
            btn_prev: btn_prev.clone(),
            btn_play_pause: btn_play_pause.clone(),
            btn_next: btn_next.clone(),
            cpu_icon,
            ram_icon,
            idle_music_icon,
            palette: RefCell::new(theme.clone()),
            last_cpu: RefCell::new(None),
            timer_source: RefCell::new(None),
            vol_debounce: RefCell::new(None),
            media_tx,
            audio_tx: audio_tx.clone(),
        });

        // 50ms debounced volume slider (A3: reaped via .status() in background thread)
        let pane_vol = Rc::downgrade(&pane);
        vol_scale.connect_value_changed(move |scale| {
            let val = scale.value() / 100.0;
            if let Some(p) = pane_vol.upgrade() {
                if let Some(src) = p.vol_debounce.borrow_mut().take() {
                    src.remove();
                }
                let source_id =
                    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                        thread::spawn(move || {
                            let _ = Command::new("wpctl")
                                .arg("set-volume")
                                .arg("@DEFAULT_AUDIO_SINK@")
                                .arg(format!("{:.2}", val))
                                .status();
                        });
                        glib::ControlFlow::Break
                    });
                *p.vol_debounce.borrow_mut() = Some(source_id);
            }
        });

        // Audio Mute Toggle
        let tx_mute = audio_tx.clone();
        btn_mute.connect_clicked(move |_| {
            let tx = tx_mute.clone();
            thread::spawn(move || {
                let _ = Command::new("wpctl")
                    .arg("set-mute")
                    .arg("@DEFAULT_AUDIO_SINK@")
                    .arg("toggle")
                    .status();
                let muted = Self::is_wpctl_muted().ok();
                let _ = tx.send_blocking((None, muted));
            });
        });

        // Connect MPRIS controls offloaded to gio::spawn_blocking (A2)
        btn_prev.connect_clicked(|_| {
            glib::spawn_future_local(async move {
                let _ = gio::spawn_blocking(|| Self::send_mpris_cmd("previous")).await;
            });
        });
        btn_play_pause.connect_clicked(|_| {
            glib::spawn_future_local(async move {
                let _ = gio::spawn_blocking(|| Self::send_mpris_cmd("play-pause")).await;
            });
        });
        btn_next.connect_clicked(|_| {
            glib::spawn_future_local(async move {
                let _ = gio::spawn_blocking(|| Self::send_mpris_cmd("next")).await;
            });
        });

        pane
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn apply_theme(&self, theme: &ThemePalette) {
        *self.palette.borrow_mut() = theme.clone();
        icons::set_button_icon(&self.btn_prev, "skip-back", &theme.text_secondary, 16);
        icons::set_button_icon(&self.btn_play_pause, "play", &theme.text_secondary, 16);
        icons::set_button_icon(&self.btn_next, "skip-forward", &theme.text_secondary, 16);
        self.cpu_icon.queue_draw();
        self.ram_icon.queue_draw();
        self.idle_music_icon.queue_draw();
        self.refresh_audio_state();
    }

    /// Triggers its 2s /proc timer strictly on show (Option 1 Lifecycle) and refreshes volume/mute
    pub fn on_show(self: &Rc<Self>) {
        self.refresh_audio_state();
        self.tick_stats();

        if self.timer_source.borrow().is_none() {
            let pane_weak = Rc::downgrade(self);
            let source_id = glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                if let Some(p) = pane_weak.upgrade() {
                    p.tick_stats();
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            });
            *self.timer_source.borrow_mut() = Some(source_id);
        }
    }

    /// Cancels its 2s timer strictly on hide and resets last_cpu so zero CPU is drained while hidden
    pub fn on_hide(&self) {
        if let Some(src) = self.timer_source.borrow_mut().take() {
            src.remove();
        }
        *self.last_cpu.borrow_mut() = None;
    }

    pub fn refresh_audio_state(&self) {
        let tx = self.audio_tx.clone();
        thread::spawn(move || {
            let vol = Self::get_wpctl_volume().ok();
            let muted = Self::is_wpctl_muted().ok();
            let _ = tx.send_blocking((vol, muted));
        });
    }

    fn tick_stats(&self) {
        // Clock
        let now = chrono_free_now();
        self.time_lbl.set_label(&now.0);
        self.date_lbl.set_label(&now.1);

        // System resources from /proc
        if let Ok(cpu_pct) = self.read_cpu_percent() {
            self.cpu_bar.set_fraction(cpu_pct / 100.0);
            self.cpu_label.set_label(&format!("{:.0}%", cpu_pct));
        }

        if let Ok((used_mb, total_mb)) = Self::read_mem_info() {
            if total_mb > 0 {
                let ram_pct = (used_mb as f64 / total_mb as f64) * 100.0;
                self.ram_bar.set_fraction(ram_pct / 100.0);
                self.ram_label.set_label(&format!("{:.0}%", ram_pct));
            }
        }

        // Async MPRIS track metadata check
        let tx = self.media_tx.clone();
        thread::spawn(move || {
            let meta = Self::read_mpris_metadata();
            let _ = tx.send_blocking(meta);
        });
    }

    fn read_cpu_percent(&self) -> Result<f64, ()> {
        let content = fs::read_to_string("/proc/stat").map_err(|_| ())?;
        let first_line = content.lines().next().ok_or(())?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(());
        }

        let user: u64 = parts[1].parse().map_err(|_| ())?;
        let nice: u64 = parts[2].parse().map_err(|_| ())?;
        let system: u64 = parts[3].parse().map_err(|_| ())?;
        let idle: u64 = parts[4].parse().map_err(|_| ())?;
        let iowait: u64 = parts.get(5).and_then(|p| p.parse().ok()).unwrap_or(0);
        let irq: u64 = parts.get(6).and_then(|p| p.parse().ok()).unwrap_or(0);
        let softirq: u64 = parts.get(7).and_then(|p| p.parse().ok()).unwrap_or(0);

        let total_idle = idle + iowait;
        let total_non_idle = user + nice + system + irq + softirq;
        let total = total_idle + total_non_idle;

        let mut last_slot = self.last_cpu.borrow_mut();
        if let Some((prev_idle, prev_total)) = *last_slot {
            let total_diff = total.saturating_sub(prev_total);
            let idle_diff = total_idle.saturating_sub(prev_idle);
            *last_slot = Some((total_idle, total));

            if total_diff > 0 {
                let cpu_pct =
                    ((total_diff.saturating_sub(idle_diff)) as f64 / total_diff as f64) * 100.0;
                return Ok(cpu_pct.clamp(0.0, 100.0));
            }
        } else {
            *last_slot = Some((total_idle, total));
        }

        Ok(0.0)
    }

    fn read_mem_info() -> Result<(u64, u64), ()> {
        let content = fs::read_to_string("/proc/meminfo").map_err(|_| ())?;
        let mut total = 0;
        let mut available = 0;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total = parse_meminfo_kb(line);
            } else if line.starts_with("MemAvailable:") {
                available = parse_meminfo_kb(line);
            }
        }

        if total > 0 {
            let used = total.saturating_sub(available);
            return Ok((used / 1024, total / 1024));
        }

        Err(())
    }

    fn read_mpris_metadata() -> (String, String) {
        let out = Command::new("playerctl")
            .arg("metadata")
            .arg("--format")
            .arg("{{title}};;;{{artist}}")
            .output();

        if let Ok(o) = out {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = s.split(";;;").collect();
                if parts.len() == 2 {
                    return (parts[0].trim().to_string(), parts[1].trim().to_string());
                } else if !s.is_empty() {
                    return (s, String::new());
                }
            }
        }

        (String::new(), String::new())
    }

    fn get_wpctl_volume() -> Result<f64, ()> {
        let out = Command::new("wpctl")
            .arg("get-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .output()
            .map_err(|_| ())?;

        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(val) = parts[1].parse::<f64>() {
                    return Ok(val.clamp(0.0, 1.0));
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

fn parse_meminfo_kb(line: &str) -> u64 {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().unwrap_or(0)
    } else {
        0
    }
}

fn chrono_free_now() -> (String, String) {
    let output = Command::new("date")
        .arg("+%H:%M:%S;;;%A, %B %d, %Y")
        .output();
    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let parts: Vec<&str> = s.split(";;;").collect();
        if parts.len() == 2 {
            return (parts[0].to_string(), parts[1].to_string());
        }
    }
    (
        "00:00:00".to_string(),
        "Monday, January 01, 1970".to_string(),
    )
}
