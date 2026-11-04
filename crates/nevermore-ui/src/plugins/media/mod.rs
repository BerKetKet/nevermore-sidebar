use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "media",
    "Media Player",
    "media",
    5,
    "Local audio and video player with mpv IPC",
);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MediaState {
    pub playlist: Vec<String>,
    pub current_idx: usize,
    pub is_playing: bool,
    pub track_title: String,
    pub position_sec: f64,
    pub duration_sec: f64,
    pub volume: u32,
}

#[derive(Clone, Debug)]
pub enum MediaMsg {
    FilesFound(Vec<String>),
    StatusUpdate {
        playing: bool,
        title: String,
        pos_sec: f64,
        dur_sec: f64,
    },
    PlaybackStarted(String),
    PlaybackStopped,
}

fn get_mpv_socket() -> String {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/nevermore/mpv.sock", runtime_dir)
}

fn format_duration(sec: f64) -> String {
    let total = sec.max(0.0) as u64;
    let mins = total / 60;
    let secs = total % 60;
    format!("{}:{:02}", mins, secs)
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let sock_path = get_mpv_socket();
    let initial_state: MediaState = ctx.load_json("playback_state").unwrap_or_default();

    // 1. Header & Now Playing UI
    let status_dot = nm_status_dot(if initial_state.is_playing {
        Tone::Success
    } else {
        Tone::Muted
    });

    let initial_title = if initial_state.track_title.is_empty() {
        "IDLE".to_string()
    } else {
        initial_state.track_title.clone()
    };
    let title_label = nm_data_label(&initial_title);
    title_label.set_tone(Tone::Primary);

    let progress_row = nm_metric_row("clock", Tone::Accent);
    let init_frac = if initial_state.duration_sec > 0.0 {
        initial_state.position_sec / initial_state.duration_sec
    } else {
        0.0
    };
    progress_row.set(
        init_frac,
        &format!(
            "{} / {}",
            format_duration(initial_state.position_sec),
            format_duration(initial_state.duration_sec)
        ),
    );

    let prev_btn = nm_icon_button("skip-back", "Previous track");
    let play_toggle = nm_toggle_icon_button("play", "pause", "Play or pause");
    let next_btn = nm_icon_button("skip-forward", "Next track");
    let vol_btn = nm_icon_button("volume-2", "Mute / Unmute");

    let controls_toolbar = nm_toolbar(
        [
            prev_btn.widget().clone().upcast::<gtk4::Widget>(),
            play_toggle.widget().clone().upcast::<gtk4::Widget>(),
            next_btn.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [vol_btn.widget().clone().upcast::<gtk4::Widget>()],
    );

    let now_playing_card = nm_card([
        status_dot.widget().clone().upcast::<gtk4::Widget>(),
        title_label.widget().clone().upcast::<gtk4::Widget>(),
        progress_row.widget().clone().upcast::<gtk4::Widget>(),
        controls_toolbar.upcast::<gtk4::Widget>(),
    ]);

    // 2. Playlist / Files UI
    let playlist_box = nm_list();
    let playlist_card = nm_card([playlist_box.clone().upcast::<gtk4::Widget>()]);

    // 3. Mailbox Handler
    let dot_for_msg = status_dot;
    let title_for_msg = title_label;
    let progress_for_msg = progress_row;
    let list_for_msg = playlist_box;
    let ctx_save = ctx.clone();

    let tx = ctx.mailbox(move |msg: MediaMsg| match msg {
        MediaMsg::FilesFound(files) => {
            while let Some(child) = list_for_msg.first_child() {
                list_for_msg.remove(&child);
            }
            if files.is_empty() {
                let empty = nm_empty_state("music", "No local media files discovered");
                let row = gtk4::ListBoxRow::new();
                row.set_child(Some(&empty));
                list_for_msg.append(&row);
            } else {
                for file_path in files.iter().take(8) {
                    let filename = std::path::Path::new(file_path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "track".to_string());
                    let play_action = nm_icon_button("play", "Play track");
                    let p_clone = file_path.clone();
                    let sock_clone = get_mpv_socket();
                    play_action.on_click(move || {
                        let cmd = Cmd::new("mpv")
                            .arg("--hwdec=vaapi")
                            .arg("--vo=gpu")
                            .arg(format!("--input-ipc-server={}", sock_clone))
                            .arg(&p_clone);
                        let _ = cmd.execute_blocking();
                    });
                    let item_row = nm_list_row(
                        "music",
                        &filename,
                        Some("LOCAL"),
                        [play_action.widget().clone().upcast::<gtk4::Widget>()],
                    );
                    list_for_msg.append(&item_row);
                }
            }
        }
        MediaMsg::StatusUpdate {
            playing,
            title,
            pos_sec,
            dur_sec,
        } => {
            if playing {
                dot_for_msg.set_tone(Tone::Success);
            } else {
                dot_for_msg.set_tone(Tone::Muted);
            }
            title_for_msg.set(&title);
            let frac = if dur_sec > 0.0 {
                (pos_sec / dur_sec).clamp(0.0, 1.0)
            } else {
                0.0
            };
            progress_for_msg.set(
                frac,
                &format!("{} / {}", format_duration(pos_sec), format_duration(dur_sec)),
            );

            let state_to_save = MediaState {
                playlist: Vec::new(),
                current_idx: 0,
                is_playing: playing,
                track_title: title,
                position_sec: pos_sec,
                duration_sec: dur_sec,
                volume: 100,
            };
            let _ = ctx_save.save_json("playback_state", &state_to_save);
        }
        MediaMsg::PlaybackStarted(name) => {
            dot_for_msg.set_tone(Tone::Success);
            title_for_msg.set(&name);
        }
        MediaMsg::PlaybackStopped => {
            dot_for_msg.set_tone(Tone::Muted);
            title_for_msg.set("PAUSED");
        }
    });

    // 4. Initial File Scan
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/bgg".to_string());
    let music_dir = format!("{}/Music", home);
    let videos_dir = format!("{}/Videos", home);
    let scan_tx = tx.clone();
    let scan_cmd = Cmd::new("find")
        .arg(&music_dir)
        .arg(&videos_dir)
        .arg("-maxdepth")
        .arg("3")
        .arg("-type")
        .arg("f")
        .arg("(")
        .arg("-name")
        .arg("*.mp3")
        .arg("-o")
        .arg("-name")
        .arg("*.flac")
        .arg("-o")
        .arg("-name")
        .arg("*.mp4")
        .arg("-o")
        .arg("-name")
        .arg("*.mkv")
        .arg("-o")
        .arg("-name")
        .arg("*.webm")
        .arg(")")
        .timeout(Duration::from_secs(3));

    ctx.run(scan_cmd, scan_tx, |res| {
        let files: Vec<String> = res
            .stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        MediaMsg::FilesFound(files)
    });

    // 5. Search Bar for Custom Paths
    let search_tx = tx.clone();
    let ctx_search = ctx.clone();
    let search_bar = nm_search_entry("Scan path or filter media", move |query| {
        if query.is_empty() {
            return;
        }
        let q_clone = query.to_string();
        let cmd = Cmd::new("find")
            .arg(&q_clone)
            .arg("-maxdepth")
            .arg("2")
            .arg("-type")
            .arg("f")
            .timeout(Duration::from_secs(3));
        let tx_clone = search_tx.clone();
        ctx_search.run(cmd, tx_clone, |res| {
            let files: Vec<String> = res
                .stdout
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            MediaMsg::FilesFound(files)
        });
    });

    // 6. Action Button Handlers
    let sock_for_toggle = sock_path.clone();
    play_toggle.on_toggle(move |active| {
        let pause_val = if active { "false" } else { "true" };
        let cmd = Cmd::new("socat")
            .arg("-")
            .arg(&sock_for_toggle)
            .stdin_bytes(format!(
                "{{\"command\": [\"set_property\", \"pause\", {}]}}\n",
                pause_val
            ));
        let _ = cmd.execute_blocking();
    });

    let sock_for_prev = sock_path.clone();
    prev_btn.on_click(move || {
        let cmd = Cmd::new("socat")
            .arg("-")
            .arg(&sock_for_prev)
            .stdin_bytes("{\"command\": [\"playlist-prev\"]}\n");
        let _ = cmd.execute_blocking();
    });

    let sock_for_next = sock_path.clone();
    next_btn.on_click(move || {
        let cmd = Cmd::new("socat")
            .arg("-")
            .arg(&sock_for_next)
            .stdin_bytes("{\"command\": [\"playlist-next\"]}\n");
        let _ = cmd.execute_blocking();
    });

    // 7. Periodic IPC Status Query (Paused when tab hidden)
    let poll_tx = tx;
    let sock_for_poll = sock_path;
    let ctx_poll = ctx.clone();
    ctx.every(Duration::from_millis(1000), move || {
        if !std::path::Path::new(&sock_for_poll).exists() {
            return;
        }
        let cmd = Cmd::new("socat")
            .arg("-")
            .arg(&sock_for_poll)
            .stdin_bytes("{\"command\": [\"get_property\", \"pause\"]}\n{\"command\": [\"get_property\", \"time-pos\"]}\n{\"command\": [\"get_property\", \"duration\"]}\n{\"command\": [\"get_property\", \"media-title\"]}\n")
            .timeout(Duration::from_millis(400));
        let tx_inner = poll_tx.clone();
        ctx_poll.run(cmd, tx_inner, |res| {
            if res.code == Some(0) {
                let lines: Vec<&str> = res.stdout.lines().collect();
                let paused = lines.first().and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok()).and_then(|v| v["data"].as_bool()).unwrap_or(true);
                let pos = lines.get(1).and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok()).and_then(|v| v["data"].as_f64()).unwrap_or(0.0);
                let dur = lines.get(2).and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok()).and_then(|v| v["data"].as_f64()).unwrap_or(0.0);
                let title = lines.get(3).and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok()).and_then(|v| v["data"].as_str().map(|s| s.to_string())).unwrap_or_else(|| "TRACK".to_string());
                MediaMsg::StatusUpdate {
                    playing: !paused,
                    title,
                    pos_sec: pos,
                    dur_sec: dur,
                }
            } else {
                MediaMsg::StatusUpdate {
                    playing: false,
                    title: "STOPPED".to_string(),
                    pos_sec: 0.0,
                    dur_sec: 0.0,
                }
            }
        });
    });

    let page = nm_page([
        search_bar.upcast::<gtk4::Widget>(),
        now_playing_card.upcast::<gtk4::Widget>(),
        playlist_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
