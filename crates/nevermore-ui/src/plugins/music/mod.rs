use nevermore_ui::prelude::*;
use nevermore_ui::sdk::mpd::{start_idle, IdleHandle};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new("socket", "MPD Socket Path", SettingKind::Path),
    SettingSpec::new("host", "MPD Host", SettingKind::Text),
    SettingSpec::new("port", "MPD Port", SettingKind::Int),
    SettingSpec::new("music_dir", "Music Directory", SettingKind::Path),
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

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum MusicMsg {
    StateUpdated {
        status: Status,
        song: Option<Song>,
        cover_bytes: Option<Vec<u8>>,
    },
    InterpolateTick,
    RefreshTriggered,
    SeekCompleted,
    MpdUnreachable(String),
}

fn resolve_endpoint(ctx: &PluginContext) -> MpdEndpoint {
    if let Some(sock) = ctx.setting_str("socket") {
        let p = PathBuf::from(sock);
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
    let endpoint = resolve_endpoint(ctx);

    // 1. Top Toolbar (Status Dot, Label, Refresh button)
    let status_dot = nm_status_dot(Tone::Secondary);
    let status_label = nm_data_label("DISCONNECTED");
    status_label.set_tone(Tone::Secondary);

    let refresh_btn = nm_icon_button("refresh-cw", "Reconnect / Refresh MPD");

    let header_toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            status_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [refresh_btn.widget().clone().upcast::<gtk4::Widget>()],
    );
    let header_card = nm_card([header_toolbar.upcast::<gtk4::Widget>()]);

    // 2. Cover Art Box (260px square)
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

    // 3. Track Metadata (Title, Artist, Album)
    let title_label = nm_data_label("No Track Playing");
    title_label.set_tone(Tone::Primary);
    title_label.widget().set_ellipsize(pango::EllipsizeMode::End);

    let artist_label = nm_data_label("Unknown Artist");
    artist_label.set_tone(Tone::Secondary);
    artist_label.widget().set_ellipsize(pango::EllipsizeMode::End);

    let album_label = nm_data_label("Unknown Album");
    album_label.set_tone(Tone::Muted);
    album_label.widget().set_ellipsize(pango::EllipsizeMode::End);

    let meta_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    meta_box.set_halign(gtk4::Align::Center);
    meta_box.append(title_label.widget());
    meta_box.append(artist_label.widget());
    meta_box.append(album_label.widget());

    // 4. Seek Bar & Time Labels
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

    // 5. Controls (Prev, Play/Pause, Next)
    let prev_btn = nm_icon_button("skip-back", "Previous Track");
    let play_pause_btn = nm_icon_button("play", "Play / Pause");
    let next_btn = nm_icon_button("skip-forward", "Next Track");

    let controls_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    controls_row.set_halign(gtk4::Align::Center);
    controls_row.append(prev_btn.widget());
    controls_row.append(play_pause_btn.widget());
    controls_row.append(next_btn.widget());

    // 6. Volume Row (Mute toggle, Volume Slider)
    let mute_btn = nm_icon_button("volume-2", "Mute / Unmute Volume");
    let vol_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 100.0, 1.0);
    vol_scale.set_hexpand(true);
    vol_scale.set_draw_value(false);
    vol_scale.set_value(70.0);

    let vol_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    vol_row.append(mute_btn.widget());
    vol_row.append(&vol_scale);

    // Player Card containing all playback items
    let player_card = nm_card([
        cover_container.clone().upcast::<gtk4::Widget>(),
        meta_box.upcast::<gtk4::Widget>(),
        seek_row.upcast::<gtk4::Widget>(),
        controls_row.upcast::<gtk4::Widget>(),
        vol_row.upcast::<gtk4::Widget>(),
    ]);

    // Unreachable Empty State
    let unreachable_view = nm_empty_state_with_tone("plug-zap", Tone::Warning, "MPD Unreachable");

    // Content container holding either player_card or unreachable_view
    let content_stack = gtk4::Stack::new();
    content_stack.add_named(&player_card, Some("player"));
    content_stack.add_named(&unreachable_view, Some("unreachable"));
    content_stack.set_visible_child_name("player");

    // State trackers on UI thread
    let cover_lru = Rc::new(RefCell::new(CoverLru::new(32)));
    let current_status = Rc::new(RefCell::new(Status::default()));
    let last_known_uri = Rc::new(RefCell::new(String::new()));
    let is_user_seeking = Rc::new(RefCell::new(false));
    let previous_volume = Rc::new(RefCell::new(70i32));
    let idle_handle: Rc<RefCell<Option<IdleHandle>>> = Rc::new(RefCell::new(None));

    let dot_for_msg = status_dot;
    let status_for_msg = status_label;
    let stack_for_msg = content_stack.clone();
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
    let lru_for_msg = cover_lru.clone();
    let cur_status_for_msg = current_status.clone();
    let uri_for_msg = last_known_uri.clone();
    let user_seeking_for_msg = is_user_seeking.clone();
    let ep_for_msg = endpoint.clone();

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
            stack_for_msg.set_visible_child_name("player");

            // Play / Pause icon
            let is_playing = status.state == PlayState::Playing;
            play_btn_clone.set_icon(if is_playing { "pause" } else { "play" });

            // Volume
            if status.volume >= 0 {
                vol_scale_clone.set_value(status.volume as f64);
                mute_btn_clone.set_icon(if status.volume == 0 {
                    "volume-x"
                } else {
                    "volume-2"
                });
            }

            // Seek slider and duration
            if !*user_seeking_for_msg.borrow() {
                seek_scale_clone.set_range(0.0, status.duration_sec.max(1.0));
                seek_scale_clone.set_value(status.elapsed_sec);
                elapsed_lbl.set(&format_time(status.elapsed_sec));
                duration_lbl.set(&format_time(status.duration_sec));
            }

            // Song metadata & Cover art
            if let Some(s) = song {
                title_lbl.set(s.title.as_deref().unwrap_or("Unknown Title"));
                artist_lbl.set(s.artist.as_deref().unwrap_or("Unknown Artist"));
                album_lbl.set(s.album.as_deref().unwrap_or("Unknown Album"));

                let uri = s.uri.clone();
                let prev_uri = uri_for_msg.borrow().clone();
                if uri != prev_uri || cover_bytes.is_some() {
                    *uri_for_msg.borrow_mut() = uri.clone();

                    // Check LRU cache
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
        MusicMsg::InterpolateTick => {
            let mut st = cur_status_for_msg.borrow_mut();
            if st.state == PlayState::Playing {
                st.elapsed_sec = (st.elapsed_sec + 1.0).min(st.duration_sec);
                if !*user_seeking_for_msg.borrow() {
                    seek_scale_clone.set_value(st.elapsed_sec);
                    elapsed_lbl.set(&format_time(st.elapsed_sec));
                }
            }
        }
        MusicMsg::RefreshTriggered => {
            let ep = ep_for_msg.clone();
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

                tx_out.send(MusicMsg::StateUpdated {
                    status,
                    song,
                    cover_bytes,
                });
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

    // 1. Refresh Button
    let tx_refresh = tx.clone();
    refresh_btn.on_click(move || {
        tx_refresh.send(MusicMsg::RefreshTriggered);
    });

    // 2. Play / Pause Button
    let ep_play = endpoint.clone();
    let tx_play = tx.clone();
    play_pause_btn.on_click(move || {
        spawn_action(ep_play.clone(), tx_play.clone(), |c| c.play_pause());
    });

    // 3. Prev Button
    let ep_prev = endpoint.clone();
    let tx_prev = tx.clone();
    prev_btn.on_click(move || {
        spawn_action(ep_prev.clone(), tx_prev.clone(), |c| c.prev());
    });

    // 4. Next Button
    let ep_next = endpoint.clone();
    let tx_next = tx.clone();
    next_btn.on_click(move || {
        spawn_action(ep_next.clone(), tx_next.clone(), |c| c.next());
    });

    // 5. Seek Slider Change
    let is_seeking_track = is_user_seeking;
    let ep_seek = endpoint.clone();
    let tx_seek = tx.clone();
    seek_scale.connect_change_value({
        let is_seeking = is_seeking_track;
        move |_, _, target_val| {
            *is_seeking.borrow_mut() = true;
            let ep = ep_seek.clone();
            let tx_action = tx_seek.clone();
            std::thread::spawn(move || {
                if let Ok(mut client) = MpdClient::connect(&ep) {
                    let _ = client.seek_cur(target_val);
                }
                tx_action.send(MusicMsg::SeekCompleted);
                tx_action.send(MusicMsg::RefreshTriggered);
            });
            glib::Propagation::Proceed
        }
    });

    // 6. Volume Slider Change
    let ep_vol = endpoint.clone();
    let tx_vol = tx.clone();
    vol_scale.connect_change_value(move |_, _, target_val| {
        let vol = target_val.clamp(0.0, 100.0) as i32;
        let ep = ep_vol.clone();
        let tx_out = tx_vol.clone();
        std::thread::spawn(move || {
            if let Ok(mut client) = MpdClient::connect(&ep) {
                let _ = client.set_volume(vol);
                tx_out.send(MusicMsg::RefreshTriggered);
            }
        });
        glib::Propagation::Proceed
    });

    // 7. Mute Button Toggle
    let prev_vol_track = previous_volume;
    let cur_status_mute = current_status;
    let ep_mute = endpoint.clone();
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

        let ep = ep_mute.clone();
        let tx_out = tx_mute.clone();
        std::thread::spawn(move || {
            if let Ok(mut client) = MpdClient::connect(&ep) {
                let _ = client.set_volume(target_vol);
                tx_out.send(MusicMsg::RefreshTriggered);
            }
        });
    });

    // 8. 1-second monotonic interpolation timer (runs ONLY when visible!)
    let tx_tick = tx.clone();
    ctx.every(Duration::from_secs(1), move || {
        tx_tick.send(MusicMsg::InterpolateTick);
    });

    // 9. Idle event listener and Show/Hide hooks
    let idle_handle_track = idle_handle;
    let ep_for_idle = endpoint;
    let tx_idle_event = tx.clone();

    let start_idle_listener = {
        let idle_holder = idle_handle_track.clone();
        let ep = ep_for_idle.clone();
        let tx_event = tx_idle_event.clone();
        move || {
            let (rx, handle) = start_idle(
                ep.clone(),
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

    // Initial start
    start_idle_listener();
    tx.send(MusicMsg::RefreshTriggered);

    let page = nm_page([
        header_card.upcast::<gtk4::Widget>(),
        content_stack.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
