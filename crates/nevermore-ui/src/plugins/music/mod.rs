pub mod lyrics;

use nevermore_core::WindowRaiser;
use nevermore_ui::prelude::*;
use nevermore_ui::sdk::mpd::{start_idle, IdleHandle};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new(
        "socket",
        "MPD Socket Path",
        SettingKind::Path {
            default: "~/.config/mpd/socket",
        },
    ),
    SettingSpec::new(
        "host",
        "MPD Host",
        SettingKind::Text {
            default: "127.0.0.1",
        },
    ),
    SettingSpec::new(
        "port",
        "MPD Port",
        SettingKind::Int {
            default: 6600,
            min: 1,
            max: 65535,
        },
    ),
    SettingSpec::new(
        "music_dir",
        "Music Directory",
        SettingKind::Path {
            default: "~/Music",
        },
    ),
];

pub const SPEC: PluginSpec = PluginSpec::with_settings(
    "music",
    "Music",
    "music",
    5,
    "MPD music player controller with live album art",
    SETTINGS,
    None,
);

#[derive(Serialize, Deserialize, Default)]
struct MusicUiState {
    subview: usize,
}

type LyricsEntry = (Option<f64>, DataLabel);

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum MusicMsg {
    StateUpdated {
        status: Status,
        song: Option<Song>,
        cover_bytes: Option<Vec<u8>>,
    },
    QueueUpdated {
        songs: Vec<Song>,
        current_id: Option<u32>,
        current_pos: Option<u32>,
    },
    LyricsUpdated {
        song_uri: String,
        lines: Vec<lyrics::LrcLine>,
        is_synced: bool,
    },
    PlaylistsUpdated {
        playlists: Vec<String>,
    },
    SearchResultsUpdated {
        query: String,
        songs: Vec<Song>,
    },
    InterpolateTick,
    RefreshTriggered,
    SeekCompleted,
    MpdUnreachable(String),
}

fn expand_tilde(p: &str) -> PathBuf {
    if let Some(stripped) = p.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(p)
}

fn resolve_endpoint(ctx: &PluginContext) -> MpdEndpoint {
    if let Some(sock) = ctx.setting_str("socket") {
        let p = expand_tilde(&sock);
        if p.exists() {
            return MpdEndpoint::Unix(p);
        }
    }
    if let Some(host) = ctx.setting_str("host") {
        let port = ctx.setting_int("port").unwrap_or(6600) as u16;
        if let Ok(addr) = format!("{}:{}", host, port).parse() {
            return MpdEndpoint::Tcp(addr);
        }
    }
    MpdEndpoint::Auto
}

fn resolve_music_dir(ctx: &PluginContext) -> PathBuf {
    if let Some(dir) = ctx.setting_str("music_dir") {
        let p = expand_tilde(&dir);
        if p.exists() {
            return p;
        }
    }
    expand_tilde("~/Music")
}

fn format_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    let m = s / 60;
    let rem_s = s % 60;
    if m >= 60 {
        let h = m / 60;
        let rem_m = m % 60;
        format!("{}:{:02}:{:02}", h, rem_m, rem_s)
    } else {
        format!("{}:{:02}", m, rem_s)
    }
}

fn format_quality_badge(status: &Status, song: Option<&Song>) -> String {
    let mut parts = Vec::new();
    if let Some(s) = song {
        if let Some(ext) = s.uri.rsplit('.').next() {
            if !ext.is_empty() && ext.len() <= 5 && !ext.contains('/') {
                parts.push(ext.to_uppercase());
            }
        }
    }
    if let Some(ref audio) = status.audio {
        let segs: Vec<&str> = audio.split(':').collect();
        if segs.len() >= 2 {
            let rate = segs[0].parse::<u32>().unwrap_or(0);
            let bits = segs[1];
            let rate_str = if rate > 0 {
                if rate % 1000 == 0 {
                    format!("{}kHz", rate / 1000)
                } else {
                    format!("{:.1}kHz", rate as f64 / 1000.0)
                }
            } else {
                segs[0].to_string()
            };
            parts.push(format!("{}-bit/{}", bits, rate_str));
        }
    }
    if let Some(kbps) = status.bitrate {
        if kbps > 0 {
            parts.push(format!("{} kbps", kbps));
        }
    }
    parts.join(" • ")
}

struct CoverLru {
    capacity: usize,
    entries: VecDeque<(String, Option<gdk::Texture>)>,
}

impl CoverLru {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    fn get(&mut self, key: &str) -> Option<Option<gdk::Texture>> {
        if let Some(idx) = self.entries.iter().position(|(k, _)| k == key) {
            let item = self.entries.remove(idx).unwrap();
            let val = item.1.clone();
            self.entries.push_front(item);
            Some(val)
        } else {
            None
        }
    }

    fn put(&mut self, key: String, val: Option<gdk::Texture>) {
        if let Some(idx) = self.entries.iter().position(|(k, _)| k == &key) {
            self.entries.remove(idx);
        } else if self.entries.len() >= self.capacity {
            self.entries.pop_back();
        }
        self.entries.push_front((key, val));
    }
}

fn spawn_action<F>(ep: MpdEndpoint, tx: Tx<MusicMsg>, action: F)
where
    F: FnOnce(&mut MpdClient) -> Result<(), MpdError> + Send + 'static,
{
    std::thread::spawn(move || {
        if let Ok(mut client) = MpdClient::connect(&ep) {
            let _ = action(&mut client);
            tx.send(MusicMsg::RefreshTriggered);
        }
    });
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let saved_state = ctx.load_json::<MusicUiState>("music_ui_state.json");
    let initial_subview = saved_state.subview.min(3);

    // 1. Top Bar: Status dot, Status label, 4-way switcher, Euphonica button, Refresh button
    let status_dot = nm_status_dot(Tone::Secondary);
    let status_label = nm_data_label("DISCONNECTED");
    status_label.set_tone(Tone::Secondary);

    let subviews = vec![
        ("disc-3", "Now Playing"),
        ("list-music", "Play Queue"),
        ("mic-vocal", "Synchronized Lyrics"),
        ("library", "Playlists & Library Search"),
    ];

    let subview_stack = gtk4::Stack::new();
    subview_stack.set_vexpand(true);
    subview_stack.set_hexpand(true);

    let ctx_subview = ctx.clone();
    let stack_subview = subview_stack.clone();
    let subview_control = nm_segmented_control(subviews, initial_subview, move |idx| {
        let name = match idx {
            1 => "queue",
            2 => "lyrics",
            3 => "library",
            _ => "now_playing",
        };
        stack_subview.set_visible_child_name(name);
        let _ = ctx_subview.save_json("music_ui_state.json", &MusicUiState { subview: idx });
    });

    let euphonica_btn = nm_icon_button("external-link", "Open Euphonica");
    let refresh_btn = nm_icon_button("refresh-cw", "Reconnect / Refresh MPD");

    let header_toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            status_label.widget().clone().upcast::<gtk4::Widget>(),
            subview_control.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [
            euphonica_btn.widget().clone().upcast::<gtk4::Widget>(),
            refresh_btn.widget().clone().upcast::<gtk4::Widget>(),
        ],
    );
    let header_card = nm_card([header_toolbar.upcast::<gtk4::Widget>()]);

    // ==========================================
    // VIEW 1: NOW PLAYING
    // ==========================================
    let cover_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    cover_container.set_halign(gtk4::Align::Center);
    cover_container.set_valign(gtk4::Align::Center);
    cover_container.set_size_request(260, 260);
    cover_container.add_css_class("nm-card");

    let placeholder_icon = nm_icon("disc-3", Tone::Muted, 96);
    placeholder_icon.widget().set_halign(gtk4::Align::Center);
    placeholder_icon.widget().set_valign(gtk4::Align::Center);
    placeholder_icon.widget().set_vexpand(true);
    placeholder_icon.widget().set_hexpand(true);
    cover_container.append(placeholder_icon.widget());

    let title_label = nm_data_label("No Track Playing");
    title_label.set_tone(Tone::Primary);
    title_label.widget().set_ellipsize(pango::EllipsizeMode::End);

    let artist_label = nm_data_label("Unknown Artist");
    artist_label.set_tone(Tone::Secondary);
    artist_label.widget().set_ellipsize(pango::EllipsizeMode::End);

    let album_label = nm_data_label("Unknown Album");
    album_label.set_tone(Tone::Muted);
    album_label.widget().set_ellipsize(pango::EllipsizeMode::End);

    let quality_badge = nm_data_label("");
    quality_badge.set_tone(Tone::Muted);
    quality_badge
        .widget()
        .set_ellipsize(pango::EllipsizeMode::End);

    let meta_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    meta_box.set_halign(gtk4::Align::Center);
    meta_box.append(title_label.widget());
    meta_box.append(artist_label.widget());
    meta_box.append(album_label.widget());
    meta_box.append(quality_badge.widget());

    let elapsed_label = nm_data_label("0:00");
    elapsed_label.set_tone(Tone::Secondary);

    let duration_label = nm_data_label("0:00");
    duration_label.set_tone(Tone::Muted);

    let seek_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0);
    seek_scale.set_hexpand(true);
    seek_scale.set_draw_value(false);

    let seek_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    seek_row.append(elapsed_label.widget());
    seek_row.append(&seek_scale);
    seek_row.append(duration_label.widget());

    // 4 Playback Mode Toggles
    let shuffle_btn = nm_toggle_icon_button("shuffle", "shuffle", "Shuffle");
    let repeat_btn = nm_toggle_icon_button("repeat", "repeat", "Repeat");
    let single_btn = nm_toggle_icon_button("repeat-1", "repeat-1", "Single Track");
    let consume_btn = nm_toggle_icon_button("trash-2", "trash-2", "Consume Mode");

    let mode_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    mode_row.set_halign(gtk4::Align::Center);
    mode_row.append(shuffle_btn.widget());
    mode_row.append(repeat_btn.widget());
    mode_row.append(single_btn.widget());
    mode_row.append(consume_btn.widget());

    // Controls
    let prev_btn = nm_icon_button("skip-back", "Previous Track");
    let play_pause_btn = nm_icon_button("play", "Play / Pause");
    let next_btn = nm_icon_button("skip-forward", "Next Track");

    let controls_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    controls_row.set_halign(gtk4::Align::Center);
    controls_row.append(prev_btn.widget());
    controls_row.append(play_pause_btn.widget());
    controls_row.append(next_btn.widget());

    // Volume
    let mute_btn = nm_icon_button("volume-2", "Mute / Unmute Volume");
    let vol_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0);
    vol_scale.set_hexpand(true);
    vol_scale.set_draw_value(false);
    vol_scale.set_value(70.0);

    let vol_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    vol_row.append(mute_btn.widget());
    vol_row.append(&vol_scale);

    let player_card = nm_card([
        cover_container.clone().upcast::<gtk4::Widget>(),
        meta_box.upcast::<gtk4::Widget>(),
        seek_row.upcast::<gtk4::Widget>(),
        mode_row.upcast::<gtk4::Widget>(),
        controls_row.upcast::<gtk4::Widget>(),
        vol_row.upcast::<gtk4::Widget>(),
    ]);

    // ==========================================
    // VIEW 2: QUEUE
    // ==========================================
    let queue_info_label = nm_data_label("0 tracks • 0:00");
    queue_info_label.set_tone(Tone::Secondary);

    let save_queue_btn = nm_icon_button("save", "Save Queue as Playlist");
    let shuffle_queue_btn = nm_icon_button("shuffle", "Shuffle Queue");
    let clear_queue_btn = nm_icon_button("trash-2", "Clear Queue");

    let queue_toolbar = nm_toolbar(
        [queue_info_label.widget().clone().upcast::<gtk4::Widget>()],
        [
            save_queue_btn.widget().clone().upcast::<gtk4::Widget>(),
            shuffle_queue_btn.widget().clone().upcast::<gtk4::Widget>(),
            clear_queue_btn.widget().clone().upcast::<gtk4::Widget>(),
        ],
    );

    let save_drawer_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    save_drawer_box.set_visible(false);

    let queue_header_card = nm_card([
        queue_toolbar.upcast::<gtk4::Widget>(),
        save_drawer_box.clone().upcast::<gtk4::Widget>(),
    ]);

    let queue_list = nm_list();
    let queue_scroll = gtk4::ScrolledWindow::new();
    queue_scroll.set_vexpand(true);
    queue_scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    queue_scroll.set_child(Some(&queue_list));

    let queue_stack = gtk4::Stack::new();
    queue_stack.set_vexpand(true);
    queue_stack.add_named(&queue_scroll, Some("list"));
    let queue_empty = nm_empty_state_with_tone("list-music", Tone::Muted, "Queue Empty");
    queue_stack.add_named(&queue_empty, Some("empty"));
    queue_stack.set_visible_child_name("empty");

    let queue_view_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    queue_view_box.append(&queue_header_card);
    queue_view_box.append(&queue_stack);

    // ==========================================
    // VIEW 3: SYNCHRONIZED LYRICS
    // ==========================================
    let lyrics_title = nm_data_label("Lyrics");
    lyrics_title.set_tone(Tone::Primary);

    let lyrics_type_label = nm_data_label("");
    lyrics_type_label.set_tone(Tone::Muted);

    let lyrics_toolbar = nm_toolbar(
        [lyrics_title.widget().clone().upcast::<gtk4::Widget>()],
        [lyrics_type_label.widget().clone().upcast::<gtk4::Widget>()],
    );
    let lyrics_header_card = nm_card([lyrics_toolbar.upcast::<gtk4::Widget>()]);

    let lyrics_lines_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    lyrics_lines_box.set_margin_top(8);
    lyrics_lines_box.set_margin_bottom(8);
    lyrics_lines_box.set_margin_start(8);
    lyrics_lines_box.set_margin_end(8);

    let lyrics_scroll = gtk4::ScrolledWindow::new();
    lyrics_scroll.set_vexpand(true);
    lyrics_scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    lyrics_scroll.set_child(Some(&lyrics_lines_box));

    let lyrics_stack = gtk4::Stack::new();
    lyrics_stack.set_vexpand(true);
    lyrics_stack.add_named(&lyrics_scroll, Some("lyrics"));
    let lyrics_empty = nm_empty_state_with_tone("mic-off", Tone::Muted, "No Lyrics Available");
    lyrics_stack.add_named(&lyrics_empty, Some("empty"));
    lyrics_stack.set_visible_child_name("empty");

    let lyrics_view_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    lyrics_view_box.append(&lyrics_header_card);
    lyrics_view_box.append(&lyrics_stack);

    // ==========================================
    // VIEW 4: PLAYLISTS & LIBRARY SEARCH
    // ==========================================
    let playlists_list = nm_list();
    let playlists_scroll = gtk4::ScrolledWindow::new();
    playlists_scroll.set_vexpand(true);
    playlists_scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    playlists_scroll.set_child(Some(&playlists_list));

    let results_list = nm_list();
    let results_scroll = gtk4::ScrolledWindow::new();
    results_scroll.set_vexpand(true);
    results_scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
    results_scroll.set_child(Some(&results_list));

    let library_stack = gtk4::Stack::new();
    library_stack.set_vexpand(true);
    library_stack.add_named(&playlists_scroll, Some("playlists"));
    library_stack.add_named(&results_scroll, Some("results"));
    let empty_playlists =
        nm_empty_state_with_tone("library", Tone::Muted, "No Saved MPD Playlists");
    let empty_results = nm_empty_state_with_tone("search", Tone::Muted, "No Results Found");
    library_stack.add_named(&empty_playlists, Some("empty_playlists"));
    library_stack.add_named(&empty_results, Some("empty_results"));
    library_stack.set_visible_child_name("playlists");

    // Add views to subview_stack
    subview_stack.add_named(&player_card, Some("now_playing"));
    subview_stack.add_named(&queue_view_box, Some("queue"));
    subview_stack.add_named(&lyrics_view_box, Some("lyrics"));

    let library_view_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let search_container_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    library_view_box.append(&search_container_box);
    library_view_box.append(&library_stack);

    subview_stack.add_named(&library_view_box, Some("library"));

    let initial_name = match initial_subview {
        1 => "queue",
        2 => "lyrics",
        3 => "library",
        _ => "now_playing",
    };
    subview_stack.set_visible_child_name(initial_name);

    // Unreachable Empty State
    let unreachable_view = nm_empty_state_with_tone("plug-zap", Tone::Warning, "MPD Unreachable");

    // Top Content stack holding either subview_stack or unreachable_view
    let top_content_stack = gtk4::Stack::new();
    top_content_stack.set_vexpand(true);
    top_content_stack.add_named(&subview_stack, Some("content"));
    top_content_stack.add_named(&unreachable_view, Some("unreachable"));
    top_content_stack.set_visible_child_name("content");

    // State trackers on UI thread
    let cover_lru = Rc::new(RefCell::new(CoverLru::new(32)));
    let current_status = Rc::new(RefCell::new(Status::default()));
    let last_known_uri = Rc::new(RefCell::new(String::new()));
    let is_user_seeking = Rc::new(RefCell::new(false));
    let is_syncing = Rc::new(Cell::new(false));
    let previous_volume = Rc::new(RefCell::new(70i32));
    let idle_handle: Rc<RefCell<Option<IdleHandle>>> = Rc::new(RefCell::new(None));

    // Active lyrics tracking
    let stored_lyrics_lines: Rc<RefCell<Vec<LyricsEntry>>> = Rc::new(RefCell::new(Vec::new()));
    let current_active_lyric: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));

    let dot_for_msg = status_dot;
    let status_for_msg = status_label;
    let stack_for_msg = top_content_stack.clone();
    let cover_box = cover_container;
    let play_btn_clone = play_pause_btn.clone();
    let seek_scale_clone = seek_scale.clone();
    let vol_scale_clone = vol_scale.clone();
    let mute_btn_clone = mute_btn.clone();
    let elapsed_lbl = elapsed_label;
    let duration_lbl = duration_label;
    let title_lbl = title_label;
    let artist_lbl = artist_label;
    let album_lbl = album_label;
    let quality_badge_lbl = quality_badge;
    let lru_for_msg = cover_lru.clone();
    let cur_status_for_msg = current_status.clone();
    let uri_for_msg = last_known_uri.clone();
    let user_seeking_for_msg = is_user_seeking.clone();
    let is_syncing_for_msg = is_syncing.clone();
    let ctx_for_refresh = ctx.clone();

    // Mode toggles clone for silent sync
    let shuffle_toggle = shuffle_btn.clone();
    let repeat_toggle = repeat_btn.clone();
    let single_toggle = single_btn.clone();
    let consume_toggle = consume_btn.clone();

    // Queue clones
    let queue_info_lbl = queue_info_label;
    let queue_list_clone = queue_list.clone();
    let queue_stack_clone = queue_stack.clone();

    // Lyrics clones
    let lyrics_title_lbl = lyrics_title;
    let lyrics_type_lbl = lyrics_type_label;
    let lyrics_lines_box_clone = lyrics_lines_box.clone();
    let lyrics_scroll_clone = lyrics_scroll.clone();
    let lyrics_stack_clone = lyrics_stack.clone();
    let lyrics_lines_holder = stored_lyrics_lines.clone();
    let active_lyric_holder = current_active_lyric.clone();

    // Library / Playlists clones
    let library_stack_clone = library_stack.clone();
    let playlists_list_clone = playlists_list.clone();
    let results_list_clone = results_list.clone();

    let tx = ctx.mailbox(move |msg: MusicMsg, tx_mailbox| match msg {
        MusicMsg::StateUpdated {
            status,
            song,
            cover_bytes,
        } => {
            dot_for_msg.set_tone(Tone::Success);
            status_for_msg.set(match status.state {
                PlayState::Playing => "PLAYING",
                PlayState::Paused => "PAUSED",
                PlayState::Stopped => "STOPPED",
            });
            status_for_msg.set_tone(Tone::Success);
            stack_for_msg.set_visible_child_name("content");

            // Play / Pause icon
            let is_playing = status.state == PlayState::Playing;
            play_btn_clone.set_icon(if is_playing { "pause" } else { "play" });

            // Volume (guarded by is_syncing)
            if status.volume >= 0 {
                is_syncing_for_msg.set(true);
                vol_scale_clone.set_value(status.volume as f64);
                is_syncing_for_msg.set(false);
                mute_btn_clone.set_icon(if status.volume == 0 {
                    "volume-x"
                } else {
                    "volume-2"
                });
            }

            // Mode toggles
            shuffle_toggle.set_active_silent(status.random);
            repeat_toggle.set_active_silent(status.repeat);
            single_toggle.set_active_silent(status.single);
            consume_toggle.set_active_silent(status.consume);

            // Seek slider and duration (guarded by is_syncing)
            if !*user_seeking_for_msg.borrow() {
                is_syncing_for_msg.set(true);
                seek_scale_clone.set_range(0.0, status.duration_sec.max(1.0));
                seek_scale_clone.set_value(status.elapsed_sec);
                is_syncing_for_msg.set(false);
                elapsed_lbl.set(&format_time(status.elapsed_sec));
                duration_lbl.set(&format_time(status.duration_sec));
            }

            // Quality badge
            let badge_str = format_quality_badge(&status, song.as_ref());
            quality_badge_lbl.set(&badge_str);

            // Song metadata & Cover art
            if let Some(s) = song {
                title_lbl.set(s.title.as_deref().unwrap_or("Unknown Title"));
                artist_lbl.set(s.artist.as_deref().unwrap_or("Unknown Artist"));
                album_lbl.set(s.album.as_deref().unwrap_or("Unknown Album"));

                lyrics_title_lbl.set(s.title.as_deref().unwrap_or("Lyrics"));

                let uri = s.uri.clone();
                let prev_uri = uri_for_msg.borrow().clone();
                if uri != prev_uri || cover_bytes.is_some() {
                    *uri_for_msg.borrow_mut() = uri.clone();

                    let cached_tex = lru_for_msg.borrow_mut().get(&uri);
                    let tex = match cached_tex {
                        Some(t) => t,
                        None => {
                            let new_tex = cover_bytes.and_then(|bytes| {
                                let gbytes = glib::Bytes::from(&bytes);
                                gdk::Texture::from_bytes(&gbytes).ok()
                            });
                            lru_for_msg.borrow_mut().put(uri.clone(), new_tex.clone());
                            new_tex
                        }
                    };

                    while let Some(child) = cover_box.first_child() {
                        cover_box.remove(&child);
                    }

                    if let Some(texture) = tex {
                        let picture = gtk4::Picture::for_paintable(&texture);
                        picture.set_content_fit(gtk4::ContentFit::Cover);
                        picture.set_size_request(260, 260);
                        cover_box.append(&picture);
                    } else {
                        let icon = nm_icon("disc-3", Tone::Muted, 96);
                        icon.widget().set_halign(gtk4::Align::Center);
                        icon.widget().set_valign(gtk4::Align::Center);
                        icon.widget().set_vexpand(true);
                        icon.widget().set_hexpand(true);
                        cover_box.append(icon.widget());
                    }
                }
            } else {
                title_lbl.set("No Track Playing");
                artist_lbl.set("Unknown Artist");
                album_lbl.set("Unknown Album");
                quality_badge_lbl.set("");
                lyrics_title_lbl.set("Lyrics");
                *uri_for_msg.borrow_mut() = String::new();

                while let Some(child) = cover_box.first_child() {
                    cover_box.remove(&child);
                }
                let icon = nm_icon("disc-3", Tone::Muted, 96);
                icon.widget().set_halign(gtk4::Align::Center);
                icon.widget().set_valign(gtk4::Align::Center);
                icon.widget().set_vexpand(true);
                icon.widget().set_hexpand(true);
                cover_box.append(icon.widget());
            }

            *cur_status_for_msg.borrow_mut() = status;
        }
        MusicMsg::QueueUpdated {
            songs,
            current_id,
            current_pos,
        } => {
            while let Some(child) = queue_list_clone.first_child() {
                queue_list_clone.remove(&child);
            }

            if songs.is_empty() {
                queue_stack_clone.set_visible_child_name("empty");
                queue_info_lbl.set("0 tracks • 0:00");
            } else {
                let total_duration: f64 = songs.iter().map(|s| s.duration_sec).sum();
                queue_info_lbl.set(&format!(
                    "{} tracks • {}",
                    songs.len(),
                    format_time(total_duration)
                ));

                let ep = resolve_endpoint(&ctx_for_refresh);
                for s in songs {
                    let song_id = s.id.unwrap_or(0);
                    let is_active = Some(song_id) == current_id && current_id.is_some();

                    let play_item_btn = nm_icon_button("play", "Play Track");
                    let next_item_btn = nm_icon_button("list-plus", "Play Next");
                    let remove_item_btn = nm_icon_button("x", "Remove from Queue");

                    let ep_play = ep.clone();
                    let tx_play = tx_mailbox.clone();
                    play_item_btn.on_click(move || {
                        spawn_action(ep_play.clone(), tx_play.clone(), move |c| {
                            c.play_id(song_id)
                        });
                    });

                    let ep_next = ep.clone();
                    let tx_next = tx_mailbox.clone();
                    let song_pos_opt = s.pos;
                    next_item_btn.on_click(move || {
                        let target_pos = match (current_pos, song_pos_opt) {
                            (Some(cur_pos), Some(song_pos)) => {
                                if song_pos == cur_pos {
                                    return;
                                } else if song_pos > cur_pos {
                                    cur_pos + 1
                                } else {
                                    cur_pos
                                }
                            }
                            (Some(cur_pos), None) => cur_pos + 1,
                            (None, _) => 0,
                        };
                        spawn_action(ep_next.clone(), tx_next.clone(), move |c| {
                            c.playlist_move_id(song_id, target_pos)
                        });
                    });

                    let ep_rem = ep.clone();
                    let tx_rem = tx_mailbox.clone();
                    remove_item_btn.on_click(move || {
                        spawn_action(ep_rem.clone(), tx_rem.clone(), move |c| {
                            c.playlist_delete_id(song_id)
                        });
                    });

                    let icon_name = if is_active {
                        "volume-2"
                    } else {
                        "music-2"
                    };
                    let sec_text = format!(
                        "{} • {}",
                        s.artist.as_deref().unwrap_or("Unknown Artist"),
                        format_time(s.duration_sec)
                    );
                    let row = nm_list_row(
                        icon_name,
                        s.title.as_deref().unwrap_or("Unknown Title"),
                        Some(&sec_text),
                        [
                            play_item_btn.widget().clone().upcast::<gtk4::Widget>(),
                            next_item_btn.widget().clone().upcast::<gtk4::Widget>(),
                            remove_item_btn.widget().clone().upcast::<gtk4::Widget>(),
                        ],
                    );
                    if is_active {
                        row.add_css_class("active-track");
                    }
                    queue_list_clone.append(&row);
                }
                queue_stack_clone.set_visible_child_name("list");
            }
        }
        MusicMsg::LyricsUpdated {
            song_uri: _,
            lines,
            is_synced,
        } => {
            while let Some(child) = lyrics_lines_box_clone.first_child() {
                lyrics_lines_box_clone.remove(&child);
            }
            lyrics_lines_holder.borrow_mut().clear();
            active_lyric_holder.set(None);

            if lines.is_empty() {
                lyrics_type_lbl.set("");
                lyrics_stack_clone.set_visible_child_name("empty");
            } else {
                lyrics_type_lbl.set(if is_synced { "Synced" } else { "Plain" });
                let ep = resolve_endpoint(&ctx_for_refresh);

                for line in lines {
                    let lbl = nm_data_label(&line.text);
                    lbl.set_tone(Tone::Secondary);
                    lbl.widget().set_wrap(true);
                    lbl.widget().set_xalign(0.5);
                    lbl.widget().set_justify(gtk4::Justification::Center);

                    if let Some(ts) = line.timestamp_sec {
                        let gesture = gtk4::GestureClick::new();
                        let ep_seek = ep.clone();
                        let tx_seek = tx_mailbox.clone();
                        gesture.connect_pressed(move |_, _, _, _| {
                            spawn_action(ep_seek.clone(), tx_seek.clone(), move |c| {
                                c.seek_cur(ts)
                            });
                        });
                        lbl.widget().add_controller(gesture);
                    }

                    lyrics_lines_holder
                        .borrow_mut()
                        .push((line.timestamp_sec, lbl.clone()));
                    lyrics_lines_box_clone.append(lbl.widget());
                }
                lyrics_stack_clone.set_visible_child_name("lyrics");
            }
        }
        MusicMsg::PlaylistsUpdated { playlists } => {
            while let Some(child) = playlists_list_clone.first_child() {
                playlists_list_clone.remove(&child);
            }

            if playlists.is_empty() {
                library_stack_clone.set_visible_child_name("empty_playlists");
            } else {
                let ep = resolve_endpoint(&ctx_for_refresh);
                for name in playlists {
                    let play_pl_btn = nm_icon_button("play", "Load & Play Playlist");
                    let append_pl_btn = nm_icon_button("plus", "Append to Queue");
                    let delete_pl_btn = nm_icon_button("trash-2", "Delete Playlist");

                    let ep_play = ep.clone();
                    let tx_play = tx_mailbox.clone();
                    let name_play = name.clone();
                    play_pl_btn.on_click(move || {
                        let name_c = name_play.clone();
                        spawn_action(ep_play.clone(), tx_play.clone(), move |c| {
                            c.playlist_clear()?;
                            c.playlist_load(&name_c)?;
                            c.play()
                        });
                    });

                    let ep_app = ep.clone();
                    let tx_app = tx_mailbox.clone();
                    let name_app = name.clone();
                    append_pl_btn.on_click(move || {
                        let name_c = name_app.clone();
                        spawn_action(ep_app.clone(), tx_app.clone(), move |c| {
                            c.playlist_load(&name_c)
                        });
                    });

                    let ep_del = ep.clone();
                    let tx_del = tx_mailbox.clone();
                    let name_del = name.clone();
                    let delete_confirm = Rc::new(Cell::new(false));
                    let btn_holder = delete_pl_btn.clone();
                    let ctx_del = ctx_for_refresh.clone();

                    delete_pl_btn.on_click(move || {
                        if delete_confirm.get() {
                            delete_confirm.set(false);
                            btn_holder.icon().set_tone(Tone::Secondary);
                            let name_c = name_del.clone();
                            spawn_action(ep_del.clone(), tx_del.clone(), move |c| {
                                c.playlist_rm(&name_c)
                            });
                        } else {
                            delete_confirm.set(true);
                            btn_holder.icon().set_tone(Tone::Danger);
                            let conf_clone = delete_confirm.clone();
                            let b_clone = btn_holder.clone();
                            ctx_del.after_delay(Duration::from_secs(3), move || {
                                if conf_clone.get() {
                                    conf_clone.set(false);
                                    b_clone.icon().set_tone(Tone::Secondary);
                                }
                            });
                        }
                    });

                    let row = nm_list_row(
                        "library",
                        &name,
                        None,
                        [
                            play_pl_btn.widget().clone().upcast::<gtk4::Widget>(),
                            append_pl_btn.widget().clone().upcast::<gtk4::Widget>(),
                            delete_pl_btn.widget().clone().upcast::<gtk4::Widget>(),
                        ],
                    );
                    playlists_list_clone.append(&row);
                }
                library_stack_clone.set_visible_child_name("playlists");
            }
        }
        MusicMsg::SearchResultsUpdated { query: _, songs } => {
            while let Some(child) = results_list_clone.first_child() {
                results_list_clone.remove(&child);
            }

            if songs.is_empty() {
                library_stack_clone.set_visible_child_name("empty_results");
            } else {
                let ep = resolve_endpoint(&ctx_for_refresh);
                for s in songs {
                    let uri_play = s.uri.clone();
                    let uri_next = s.uri.clone();
                    let uri_add = s.uri.clone();

                    let play_res_btn = nm_icon_button("play", "Play Now");
                    let next_res_btn = nm_icon_button("list-plus", "Play Next");
                    let add_res_btn = nm_icon_button("plus", "Add to Queue");

                    let ep_play = ep.clone();
                    let tx_play = tx_mailbox.clone();
                    play_res_btn.on_click(move || {
                        let u = uri_play.clone();
                        spawn_action(ep_play.clone(), tx_play.clone(), move |c| {
                            let id = c.add_id(&u)?;
                            c.play_id(id)
                        });
                    });

                    let ep_next = ep.clone();
                    let tx_next = tx_mailbox.clone();
                    next_res_btn.on_click(move || {
                        let u = uri_next.clone();
                        spawn_action(ep_next.clone(), tx_next.clone(), move |c| {
                            let song_pos = c.status()?.song_pos;
                            let id = c.add_id(&u)?;
                            if let Some(cur_pos) = song_pos {
                                c.playlist_move_id(id, cur_pos + 1)?;
                            }
                            Ok(())
                        });
                    });

                    let ep_add = ep.clone();
                    let tx_add = tx_mailbox.clone();
                    add_res_btn.on_click(move || {
                        let u = uri_add.clone();
                        spawn_action(ep_add.clone(), tx_add.clone(), move |c| {
                            let _ = c.add_id(&u);
                            Ok(())
                        });
                    });

                    let sec_text = format!(
                        "{} • {}",
                        s.artist.as_deref().unwrap_or("Unknown Artist"),
                        s.album.as_deref().unwrap_or("Unknown Album")
                    );
                    let row = nm_list_row(
                        "music",
                        s.title.as_deref().unwrap_or(&s.uri),
                        Some(&sec_text),
                        [
                            play_res_btn.widget().clone().upcast::<gtk4::Widget>(),
                            next_res_btn.widget().clone().upcast::<gtk4::Widget>(),
                            add_res_btn.widget().clone().upcast::<gtk4::Widget>(),
                        ],
                    );
                    results_list_clone.append(&row);
                }
                library_stack_clone.set_visible_child_name("results");
            }
        }
        MusicMsg::InterpolateTick => {
            let mut st = cur_status_for_msg.borrow_mut();
            if st.state == PlayState::Playing {
                st.elapsed_sec = (st.elapsed_sec + 1.0).min(st.duration_sec);
                if !*user_seeking_for_msg.borrow() {
                    is_syncing_for_msg.set(true);
                    seek_scale_clone.set_value(st.elapsed_sec);
                    is_syncing_for_msg.set(false);
                    elapsed_lbl.set(&format_time(st.elapsed_sec));
                }

                // Synced lyrics highlight & scroll
                let elapsed = st.elapsed_sec;
                let lines_ref = lyrics_lines_holder.borrow();
                let mut best_idx = None;
                for (idx, (ts, _)) in lines_ref.iter().enumerate() {
                    if let Some(t) = ts {
                        if *t <= elapsed {
                            best_idx = Some(idx);
                        } else {
                            break;
                        }
                    }
                }

                if let Some(idx) = best_idx {
                    if active_lyric_holder.get() != Some(idx) {
                        if let Some(prev) = active_lyric_holder.get() {
                            if let Some((_, prev_lbl)) = lines_ref.get(prev) {
                                prev_lbl.set_tone(Tone::Secondary);
                            }
                        }
                        if let Some((_, curr_lbl)) = lines_ref.get(idx) {
                            curr_lbl.set_tone(Tone::Accent);
                        }
                        active_lyric_holder.set(Some(idx));

                        let total = lines_ref.len().max(1);
                        let adj = lyrics_scroll_clone.vadjustment();
                        let target = (idx as f64 / total as f64) * (adj.upper() - adj.page_size());
                        adj.set_value(target.max(0.0));
                    }
                }
            }
        }
        MusicMsg::RefreshTriggered => {
            let ep = resolve_endpoint(&ctx_for_refresh);
            let music_dir = resolve_music_dir(&ctx_for_refresh);
            let cache_dir = ctx_for_refresh.data_dir.join("lyrics");
            let tx_out = tx_mailbox.clone();
            let cur_uri = uri_for_msg.borrow().clone();
            let in_lru = if cur_uri.is_empty() {
                true
            } else {
                lru_for_msg.borrow_mut().get(&cur_uri).is_some()
            };

            std::thread::spawn(move || {
                let mut client = match MpdClient::connect(&ep) {
                    Ok(c) => c,
                    Err(e) => {
                        tx_out.send(MusicMsg::MpdUnreachable(e.to_string()));
                        return;
                    }
                };

                let status = match client.status() {
                    Ok(s) => s,
                    Err(e) => {
                        tx_out.send(MusicMsg::MpdUnreachable(e.to_string()));
                        return;
                    }
                };

                let song = client.current_song().ok().flatten();

                let mut cover_bytes = None;
                if let Some(ref s) = song {
                    if (s.uri != cur_uri || !in_lru) && !s.uri.is_empty() {
                        if let Ok(Some((bytes, _))) = client.get_picture(&s.uri) {
                            cover_bytes = Some(bytes);
                        }
                    }
                }

                // Fetch queue
                if let Ok(queue_songs) = client.playlist_info(0, 100) {
                    tx_out.send(MusicMsg::QueueUpdated {
                        songs: queue_songs,
                        current_id: status.song_id,
                        current_pos: status.song_pos,
                    });
                }

                // Fetch playlists
                if let Ok(playlists) = client.list_playlists() {
                    tx_out.send(MusicMsg::PlaylistsUpdated { playlists });
                }

                tx_out.send(MusicMsg::StateUpdated {
                    status,
                    song: song.clone(),
                    cover_bytes,
                });

                // Lyrics
                if let Some(ref s) = song {
                    let song_uri = s.uri.clone();
                    let artist = s.artist.clone().unwrap_or_default();
                    let title = s.title.clone().unwrap_or_default();
                    let album = s.album.clone().unwrap_or_default();
                    let duration = s.duration_sec;

                    let lyrics_content = lyrics::find_local_lyrics(&music_dir, &song_uri)
                        .or_else(|| {
                            lyrics::fetch_lrclib_lyrics(
                                &cache_dir, &artist, &title, &album, duration,
                            )
                        });

                    if let Some(content) = lyrics_content {
                        let (lines, is_synced) = lyrics::parse_lrc(&content);
                        tx_out.send(MusicMsg::LyricsUpdated {
                            song_uri,
                            lines,
                            is_synced,
                        });
                    } else {
                        tx_out.send(MusicMsg::LyricsUpdated {
                            song_uri,
                            lines: Vec::new(),
                            is_synced: false,
                        });
                    }
                }
            });
        }
        MusicMsg::SeekCompleted => {
            *user_seeking_for_msg.borrow_mut() = false;
        }
        MusicMsg::MpdUnreachable(_err) => {
            dot_for_msg.set_tone(Tone::Warning);
            status_for_msg.set("UNREACHABLE");
            status_for_msg.set_tone(Tone::Warning);
            stack_for_msg.set_visible_child_name("unreachable");
        }
    });

    // 1. Refresh Button & Euphonica Button
    let tx_refresh = tx.clone();
    refresh_btn.on_click(move || {
        tx_refresh.send(MusicMsg::RefreshTriggered);
    });

    let ctx_euphonica = ctx.clone();
    let tx_euphonica = tx.clone();
    euphonica_btn.on_click(move || {
        let raised = ctx_euphonica
            .kwin
            .raise_application("io.github.htkhiem.Euphonica")
            .is_ok();
        if !raised {
            ctx_euphonica.run(Cmd::new("euphonica"), &tx_euphonica, |_| {
                MusicMsg::RefreshTriggered
            });
        }
    });

    // 2. Play / Pause Button
    let ctx_play = ctx.clone();
    let tx_play = tx.clone();
    play_pause_btn.on_click(move || {
        let ep = resolve_endpoint(&ctx_play);
        spawn_action(ep, tx_play.clone(), |c| c.play_pause());
    });

    // 3. Prev Button
    let ctx_prev = ctx.clone();
    let tx_prev = tx.clone();
    prev_btn.on_click(move || {
        let ep = resolve_endpoint(&ctx_prev);
        spawn_action(ep, tx_prev.clone(), |c| c.prev());
    });

    // 4. Next Button
    let ctx_next = ctx.clone();
    let tx_next = tx.clone();
    next_btn.on_click(move || {
        let ep = resolve_endpoint(&ctx_next);
        spawn_action(ep, tx_next.clone(), |c| c.next());
    });

    // 5. Mode Toggles
    let ctx_shuf = ctx.clone();
    let tx_shuf = tx.clone();
    shuffle_btn.on_toggle(move |val| {
        let ep = resolve_endpoint(&ctx_shuf);
        spawn_action(ep, tx_shuf.clone(), move |c| c.set_random(val));
    });

    let ctx_rep = ctx.clone();
    let tx_rep = tx.clone();
    repeat_btn.on_toggle(move |val| {
        let ep = resolve_endpoint(&ctx_rep);
        spawn_action(ep, tx_rep.clone(), move |c| c.set_repeat(val));
    });

    let ctx_sng = ctx.clone();
    let tx_sng = tx.clone();
    single_btn.on_toggle(move |val| {
        let ep = resolve_endpoint(&ctx_sng);
        spawn_action(ep, tx_sng.clone(), move |c| c.set_single(val));
    });

    let ctx_csm = ctx.clone();
    let tx_csm = tx.clone();
    consume_btn.on_toggle(move |val| {
        let ep = resolve_endpoint(&ctx_csm);
        spawn_action(ep, tx_csm.clone(), move |c| c.set_consume(val));
    });

    // 6. Queue actions: Save Queue as Playlist
    let drawer_box_ref = save_drawer_box;
    let ctx_save_pl = ctx.clone();
    let tx_save_pl = tx.clone();
    let prompt = nm_prompt_entry("save", "Playlist name...", move |name| {
        let ep = resolve_endpoint(&ctx_save_pl);
        let n = name.clone();
        spawn_action(ep, tx_save_pl.clone(), move |c| c.playlist_save(&n));
    });
    drawer_box_ref.append(&prompt);

    let drawer_toggle = drawer_box_ref;
    save_queue_btn.on_click(move || {
        let visible = drawer_toggle.is_visible();
        drawer_toggle.set_visible(!visible);
    });

    let ctx_shuf_q = ctx.clone();
    let tx_shuf_q = tx.clone();
    shuffle_queue_btn.on_click(move || {
        let ep = resolve_endpoint(&ctx_shuf_q);
        spawn_action(ep, tx_shuf_q.clone(), |c| c.playlist_shuffle());
    });

    let ctx_clear_q = ctx.clone();
    let tx_clear_q = tx.clone();
    let clear_confirming = Rc::new(Cell::new(false));
    let clear_btn_ref = clear_queue_btn.clone();
    clear_queue_btn.on_click(move || {
        if clear_confirming.get() {
            clear_confirming.set(false);
            clear_btn_ref.icon().set_tone(Tone::Secondary);
            let ep = resolve_endpoint(&ctx_clear_q);
            spawn_action(ep, tx_clear_q.clone(), |c| c.playlist_clear());
        } else {
            clear_confirming.set(true);
            clear_btn_ref.icon().set_tone(Tone::Danger);
            let conf_clone = clear_confirming.clone();
            let b_clone = clear_btn_ref.clone();
            ctx_clear_q.after_delay(Duration::from_secs(3), move || {
                if conf_clone.get() {
                    conf_clone.set(false);
                    b_clone.icon().set_tone(Tone::Secondary);
                }
            });
        }
    });

    // 7. Library Search Entry
    let ctx_search = ctx.clone();
    let tx_search = tx.clone();
    let search_entry = nm_search_entry("Search artist, album, track...", move |query| {
        let trimmed = query.trim().to_string();
        let ep = resolve_endpoint(&ctx_search);
        let tx_out = tx_search.clone();
        if trimmed.len() >= 2 {
            std::thread::spawn(move || {
                if let Ok(mut client) = MpdClient::connect(&ep) {
                    if let Ok(songs) = client.search(&trimmed, 0, 50) {
                        tx_out.send(MusicMsg::SearchResultsUpdated {
                            query: trimmed,
                            songs,
                        });
                    }
                }
            });
        } else {
            std::thread::spawn(move || {
                if let Ok(mut client) = MpdClient::connect(&ep) {
                    if let Ok(playlists) = client.list_playlists() {
                        tx_out.send(MusicMsg::PlaylistsUpdated { playlists });
                    }
                }
            });
        }
    });
    search_container_box.append(&search_entry);

    // 8. Seek Slider Change (Guarded by is_syncing, debounced 100ms)
    let is_seeking_track = is_user_seeking;
    let is_syncing_seek = is_syncing.clone();
    let ctx_seek = ctx.clone();
    let tx_seek = tx.clone();
    let seek_debounce: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    seek_scale.connect_change_value({
        let is_seeking = is_seeking_track;
        let is_sync = is_syncing_seek;
        let debounce = seek_debounce;
        move |_, _, target_val| {
            if is_sync.get() {
                return glib::Propagation::Proceed;
            }
            *is_seeking.borrow_mut() = true;
            if let Some(source) = debounce.borrow_mut().take() {
                safe_source_remove(source);
            }
            let ep = resolve_endpoint(&ctx_seek);
            let tx_action = tx_seek.clone();
            let deb_clone = debounce.clone();
            let source_id = ctx_seek.after_delay(Duration::from_millis(100), move || {
                deb_clone.borrow_mut().take();
                std::thread::spawn(move || {
                    if let Ok(mut client) = MpdClient::connect(&ep) {
                        let _ = client.seek_cur(target_val);
                    }
                    tx_action.send(MusicMsg::SeekCompleted);
                    tx_action.send(MusicMsg::RefreshTriggered);
                });
            });
            *debounce.borrow_mut() = Some(source_id);
            glib::Propagation::Proceed
        }
    });

    // 9. Volume Slider Change (Guarded by is_syncing, debounced 100ms)
    let is_syncing_vol = is_syncing;
    let ctx_vol = ctx.clone();
    let tx_vol = tx.clone();
    let vol_debounce: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    vol_scale.connect_change_value({
        let is_sync = is_syncing_vol;
        let debounce = vol_debounce;
        move |_, _, target_val| {
            if is_sync.get() {
                return glib::Propagation::Proceed;
            }
            let vol = target_val.clamp(0.0, 100.0) as i32;
            if let Some(source) = debounce.borrow_mut().take() {
                safe_source_remove(source);
            }
            let ep = resolve_endpoint(&ctx_vol);
            let tx_out = tx_vol.clone();
            let deb_clone = debounce.clone();
            let source_id = ctx_vol.after_delay(Duration::from_millis(100), move || {
                deb_clone.borrow_mut().take();
                std::thread::spawn(move || {
                    if let Ok(mut client) = MpdClient::connect(&ep) {
                        let _ = client.set_volume(vol);
                        tx_out.send(MusicMsg::RefreshTriggered);
                    }
                });
            });
            *debounce.borrow_mut() = Some(source_id);
            glib::Propagation::Proceed
        }
    });

    // 10. Mute Button Toggle
    let prev_vol_track = previous_volume;
    let cur_status_mute = current_status;
    let ctx_mute = ctx.clone();
    let tx_mute = tx.clone();
    mute_btn.on_click(move || {
        let cur_vol = cur_status_mute.borrow().volume;
        let target_vol = if cur_vol > 0 {
            *prev_vol_track.borrow_mut() = cur_vol;
            0
        } else {
            let restore = *prev_vol_track.borrow();
            restore.max(20)
        };

        let ep = resolve_endpoint(&ctx_mute);
        let tx_out = tx_mute.clone();
        std::thread::spawn(move || {
            if let Ok(mut client) = MpdClient::connect(&ep) {
                let _ = client.set_volume(target_vol);
                tx_out.send(MusicMsg::RefreshTriggered);
            }
        });
    });

    // 11. 1-second monotonic interpolation timer (runs ONLY when visible!)
    let tx_tick = tx.clone();
    ctx.every(Duration::from_secs(1), move || {
        tx_tick.send(MusicMsg::InterpolateTick);
    });

    // 12. Idle event listener and Show/Hide hooks
    let idle_handle_track = idle_handle;
    let ctx_idle = ctx.clone();
    let tx_idle_event = tx.clone();

    let start_idle_listener = {
        let idle_holder = idle_handle_track.clone();
        let ctx_ep = ctx_idle;
        let tx_event = tx_idle_event.clone();
        move || {
            let ep = resolve_endpoint(&ctx_ep);
            let (rx, handle) = start_idle(
                ep,
                &[
                    "player",
                    "playlist",
                    "mixer",
                    "options",
                    "stored_playlist",
                    "database",
                    "update",
                ],
            );
            *idle_holder.borrow_mut() = Some(handle);

            let tx_out = tx_event.clone();
            glib::spawn_future_local(async move {
                while let Ok(_events) = rx.recv().await {
                    tx_out.send(MusicMsg::RefreshTriggered);
                }
            });
        }
    };

    // On Show: start idle thread and trigger fetch
    let start_on_show = start_idle_listener.clone();
    let tx_on_show = tx.clone();
    ctx.on_show(move || {
        start_on_show();
        tx_on_show.send(MusicMsg::RefreshTriggered);
    });

    // On Hide: cleanly stop idle thread (guaranteeing 0.0% background CPU)
    let idle_for_hide = idle_handle_track;
    ctx.on_hide(move || {
        if let Some(mut handle) = idle_for_hide.borrow_mut().take() {
            handle.stop();
        }
    });

    let page = nm_page([
        header_card.upcast::<gtk4::Widget>(),
        top_content_stack.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
