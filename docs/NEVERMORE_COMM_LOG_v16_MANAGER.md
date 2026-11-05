# [MANAGER AI DIRECTIVE — `v16_MANAGER`]: REMEDIATION PASS (R1–R6) & PHASE 13 FEATURE IMPLEMENTATION (F1–F4)

**From:** Manager AI & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Base Commit on `main`:** `459e215`  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v16_MANAGER.md`  

==================================================================
COMMIT 1 — REMEDIATION PASS (R1 THROUGH R6)
==================================================================

1. R1 — Unify scripts/music-import Ongoing Import Mode with Migration Safety Rules:
   - In scripts/music-import (import_external_paths, lines 368–423), replace the shortcut implementation so ongoing imports obey the exact same safety rules as plan_musicbee_migration:
     * Never overwrite existing files: if dest_path exists with identical SHA-256 hash, skip and count as duplicate (or move to ~/Music-Inbox/duplicates/ if --move); if dest_path exists with different SHA-256 hash, divert to ~/Music-Inbox/conflicts/.
     * If required tags (artist/albumartist, album, title) are missing, quarantine to ~/Music-Inbox/needs-tags/ instead of creating "Unknown Artist" / "Unknown Album".
     * Automatically copy (or move if --move) album cover images (cover/folder/front.{jpg,png} -> cover.{jpg,png}) and matching .lrc / .cue sidecars alongside imported tracks.
   - Copy the updated scripts/music-import to /home/bgg/.local/bin/music-import and test it on a scratch directory in /tmp/test_import_r1 containing a tagged track, a duplicate track, an untagged track, a cover.jpg, and a .lrc file.

2. R2 — Fix CLI SetTheme Font Override, Active/Pinned Icon Recoloring, & NavRail Scroll Filter:
   - In crates/nevermore-ui/src/main.rs (IpcRequest::SetTheme), apply get_appearance_config() (ui_font, mono_font, mono_font_size) to new_palette before calling css_prov_ipc.load_from_string(&new_palette.to_gtk_css()).
   - In crates/nevermore-core/src/theme.rs and crates/nevermore-ui/src/icons.rs: ensure icons inside .nav-btn.active and .btn-header-pin.pinned inherit high-contrast colors via CSS (.nav-btn.active .nm-icon { color: var(--nm-accent); } and .btn-header-pin.pinned .nm-icon { color: var(--nm-accent-fg); }), and in main.rs only swap btn_pin's icon when transitioning into or out of SidebarState::Pinned rather than reallocating a new DrawingArea on every state event.
   - In crates/nevermore-ui/src/nav_rail.rs: filter tabs_order in the horizontal EventControllerScroll handler so it only cycles through tabs whose button is currently visible (btn.is_visible()).

3. R3 — Add Default Values to SettingKind (crates/nevermore-ui/src/sdk/types.rs & settings_page.rs):
   - Update SettingKind variants to carry defaults:
     * Text { default: &'static str }
     * Path { default: &'static str }
     * Int { default: i64, min: i64, max: i64 }
     * Bool { default: bool }
     * Choice { options: &'static [&'static str], default: &'static str }
     * Secret { key_name: &'static str }
   - Update settings_page.rs and PluginContext::setting_str / setting_int / setting_bool so when a key is not present in ~/.config/nevermore/plugins/<id>.toml, it falls back to the SettingSpec's declared default (e.g., port = 6600, poll_interval_sec = 120, socket = "~/.config/mpd/socket", music_dir = "~/Music").

4. R4 — Fix Music Plugin Startup Idle Thread, Slider Thread Storm, & Tilde Expansion (crates/nevermore-ui/src/plugins/music/mod.rs):
   - Remove start_idle_listener() and tx.send(MusicMsg::RefreshTriggered) from the end of build(); let on_show start them only when the Music tab becomes visible.
   - Debounce seek_scale and vol_scale user drags (100ms timer coalescing the latest slider value before spawning the background MPD command) so dragging sliders never spawns dozens of threads per second.
   - Expand leading "~/" to $HOME in resolve_endpoint and resolve_music_dir so "~/.config/mpd/socket" and "~/Music" resolve properly on disk.

5. R5 — Eliminate Main-Thread secret-tool Calls, Fix Gemini Chat Turn Order, & Add OAuth Timeout:
   - In crates/nevermore-ui/src/plugins/gemini/mod.rs and crates/nevermore-ui/src/plugins/email/mod.rs: remove all calls to secret_get_blocking / secret_set_blocking from the GTK main thread. Perform secret-tool lookups and writes strictly inside background worker threads (std::thread::spawn / gio::spawn_blocking) and cache loaded tokens in memory inside the mailbox closure.
   - In gemini/mod.rs: render conversation history in natural reading order (or grouped by (user, model) pairs so the user's prompt always appears immediately above the model's answer to that prompt), and replace &err_msg[..20] with char-safe .chars().take(20).collect::<String>().
   - In email/mod.rs: in wait_for_redirect_code, set non-blocking mode / a 120-second deadline on TcpListener::accept() and a 5-second read timeout on the accepted TcpStream so an abandoned browser tab never hangs a background thread forever.

6. R6 — Assert catch_unwind in tests/plugin_contract.rs:
   - In crates/nevermore-ui/tests/plugin_contract.rs (run_lifecycle_pump_and_cleanup), assert!(res.is_ok(), "Plugin panicked or failed timer cleanup during lifecycle test") instead of discarding let _ = res.

==================================================================
COMMIT 2 — F1: EMAIL PLUGIN UPGRADE & 500x1080 COMPANION SIDE-BLADE WINDOW
==================================================================

1. Companion Side-Blade Window (500x1080 on Layer::Overlay at margin_right = 400):
   - Provide a reusable 500x1080 companion blade window managed by the shell / PluginContext (configured with gtk4-layer-shell: Layer::Overlay, exclusive_zone = -1, KeyboardMode::OnDemand, anchored to Edge::Right | Edge::Top | Edge::Bottom, set_margin(Edge::Right, 400), set_size_request(500, monitor_height), set_default_size(500, monitor_height)).
   - Pointer-Leave Coordination: In main.rs, while the companion blade window is visible and hovered/focused, suppress panel_window's 250ms pointer-leave auto-hide timer so moving the cursor from the 400px sidebar into the 500x1080 blade never auto-hides the sidebar! Automatically hide the companion blade whenever the main sidebar transitions to SidebarState::Hidden, switches away from the Email tab, or when the user clicks the blade's Close (x) button.

2. Inbox Row Actions (crates/nevermore-ui/src/plugins/email/mod.rs):
   - Each unread email card in the 340px inbox list must provide:
     * Click / Read (mail / mail-open icon or clicking the row): Opens the full email in the 500x1080 Companion Side-Blade.
     * Reply (corner-up-left icon): Opens the email in the 500x1080 Companion Side-Blade and focuses the bottom Reply composer.
     * Mark as Read (check icon): POST /gmail/v1/users/me/messages/<id>/modify with {"removeLabelIds": ["UNREAD"]}.
     * Mark as Spam (shield-off icon): POST /gmail/v1/users/me/messages/<id>/modify with {"addLabelIds": ["SPAM"], "removeLabelIds": ["INBOX", "UNREAD"]} and removes the row.
     * Delete / Trash (trash-2 icon): POST /gmail/v1/users/me/messages/<id>/trash and removes the row.
     * Open in Browser (external-link icon): xdg-open https://mail.google.com/mail/u/0/#inbox/<threadId>.

3. Full Email Reader, Link Opener, & Inline RFC 2822 Reply Composer in the 500x1080 Blade:
   - Fetch GET https://gmail.googleapis.com/gmail/v1/users/me/messages/<id>?format=full via curl -K - stdin.
   - Recursively parse MIME payload.body.data and payload.parts (decoding Base64URL text/plain and text/html, stripping HTML tags/styles cleanly into readable paragraphs if text/plain is absent, and extracting all http/https URLs as clickable link buttons/chips that can open in an app/webview or via xdg-open). Also extract Message-ID, Subject, From, Reply-To, Date, and References headers.
   - Top Blade Toolbar: Subject (wrapped), From, Date, plus action buttons for Reply, Mark as Read, Mark as Spam, Delete/Trash, Open in Browser, and Close Blade (x).
   - Bottom Reply Drawer: Multi-line gtk4::TextView composer with a Send Reply (send icon) button. Sending constructs a valid RFC 2822 message (To: <Reply-To or From>, Subject: Re: <Subject>, In-Reply-To: <Message-ID>, References: <References> <Message-ID>, Content-Type: text/plain; charset="UTF-8", body), Base64URL-encodes it into {"raw": "<base64url>", "threadId": "<threadId>"}, and sends it via POST https://gmail.googleapis.com/gmail/v1/users/me/messages/send over curl -K - stdin.

==================================================================
COMMIT 3 — F2: FULL MPD MUSIC PLUGIN VIEWS (NOW PLAYING, QUEUE, LYRICS, PLAYLISTS & SEARCH)
==================================================================

Upgrade crates/nevermore-ui/src/plugins/music/mod.rs and crates/nevermore-ui/src/sdk/mpd.rs:
1. Top Navigation Bar:
   - 4-way nm_segmented icon switcher: Now Playing ("disc-3"), Queue ("list-music"), Lyrics ("mic-vocal"), and Playlists & Search ("library"), persisting the active sub-view via ctx.save_json("music_ui_state.json", ...).
   - Right-aligned "Open Euphonica" icon button ("external-link") that raises Euphonica via ctx.kwin.raise_application("io.github.htkhiem.Euphonica") or launches euphonica if not running.
2. View 1 — Now Playing Enhancements:
   - Keep 260px cover art, Title, Artist, Album, debounced Seek bar, transport controls, and debounced Volume bar.
   - Add a live Audio Quality Badge (.nm-data.nm-muted) under Album formatted from MPD status (audio samplerate:bits:channels and bitrate) + song file extension (e.g. "FLAC • 16-bit/44.1kHz • 945 kbps" or "MP3 • 320 kbps").
   - Add a row of 4 playback mode toggles (nm_toggle_icon_button synced via set_active_silent): Shuffle ("shuffle" -> random 0|1), Repeat ("repeat" -> repeat 0|1), Single ("repeat-1" -> single 0|1), and Consume ("trash-2" -> consume 0|1).
3. View 2 — Queue ("list-music"):
   - Query MPD playlistinfo 0:100.
   - Header toolbar: track count & total duration (.nm-data), Save Queue as Playlist ("save" toggling an nm_prompt_entry that runs MPD save "<name>"), Shuffle Queue ("shuffle" -> MPD shuffle), and Two-Step Clear Queue ("trash-2": first click sets danger tone for 3s, second click runs MPD clear).
   - Queue rows: show Title & Artist (highlighting the currently playing song in Tone::Accent) with 3 icon buttons: Play ("play" -> playid <id>), Play Next ("list-plus" -> moveid <id> to immediately after current song position), and Remove ("x" -> deleteid <id>).
4. View 3 — Synchronized Lyrics ("mic-vocal"):
   - Source 1: Check local sidecar <track_stem>.lrc or <track_stem>.txt inside music_dir (verified via canonicalize() to stay inside music_dir).
   - Source 2: If no local sidecar exists, query https://lrclib.net/api/get?artist_name=...&track_name=...&album_name=...&duration=... via Cmd::curl (percent-encoded, 8s timeout) and cache in ctx.data_dir/lyrics/<sha256>.json (using glib::Checksum).
   - Parse [mm:ss.xx] LRC timestamps, render lines as clickable .nm-data rows in a scrolled container, highlight the active line in Tone::Accent during playback, auto-scroll when the active line index changes, and seek MPD (seekcur <sec>) when any timestamped line is clicked.
5. View 4 — Playlists & Library Search ("library"):
   - Top search entry (nm_search_entry or nm_prompt_entry): when query >= 2 chars, run MPD search any "<query>" window 0:50 and render matching songs with Play Now ("play"), Play Next ("list-plus"), and Add to Queue ("plus" -> addid <uri>).
   - When search is empty, list saved MPD playlists (listplaylists) with Load & Play ("play" -> clear + load + play), Append to Queue ("plus" -> load), and Two-Step Delete ("trash-2" -> rm "<name>").

==================================================================
COMMIT 4 — F3 & F4: MULTI-NOTE NOTEPAD WITH PASTE + CLICKABLE NOTIFICATIONS
==================================================================

1. F3 — Multi-Note Notepad & Clipboard Paste (crates/nevermore-ui/src/plugins/notepad/):
   - Store multiple notes in notes.json (Vec<NoteItem { id, title, content, updated_at }>, active_id) using atomic temp-file + rename writes. On first load, if notes.json does not exist, automatically import existing notes.txt / scratchpad.txt as the first note so zero text is lost.
   - Top Toolbar:
     * New Note ("plus", tooltip "New Note"): creates a new note, switches to it, and focuses the TextView.
     * Paste from Clipboard ("list-plus" or "copy", tooltip "Paste from Clipboard"): asynchronously reads text from gdk::Display::default().clipboard().read_text_async (or wl-paste fallback) and inserts it at the cursor in the active note buffer.
     * Copy Note ("copy", tooltip "Copy Note"): copies selection (or full note text) to the Wayland clipboard.
     * Delete Note ("trash-2", tooltip "Delete Note"): two-step 3s danger confirmation, keeping at least 1 empty note.
   - Note Switcher Bar: horizontal scrollable bar of compact note buttons showing each note's title (auto-derived from first non-empty line, truncated to 18 chars, .nm-data) so switching notes takes 1 click.

2. F4 — Clickable Notification Cards (crates/nevermore-ui/src/plugins/notifications/pane.rs):
   - Attach a GestureClick to each notification card body (keeping the Dismiss "x" button separate) and add hover feedback in CSS (.notif-card:hover { border-color: var(--nm-accent); }).
   - When tapped, run off the main thread:
     * Extract any https:// or http:// URL from summary or body and open via xdg-open <url>.
     * Attempt KWin window activation via kwin.raise_application(&desktop_entry) (and fallback app_name).
     * If raise_application indicates no matching running window and desktop_entry is non-empty, launch the app via gtk-launch <desktop_entry>.

==================================================================
COMMIT 5 — CONTRACT TESTS, DOCS UPDATE, DEPLOYMENT, & FULL-SCREEN SCREENSHOTS
==================================================================

1. Update docs/ZERO_CONTEXT_LLM_PROMPT.md and README.md to reflect all R1–R6 and F1–F4 updates.
2. Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace --locked, and WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract (must pass 100% with 0 warnings).
3. Build release binaries (cargo build --release --workspace --locked), deploy to /home/bgg/.local/bin/, and restart nevermore.service.
4. Using Protocol 0.2 (5s notify-send warning -> sleep 6s -> DND Inhibit ON -> spectacle -> DND OFF -> all-clear), capture full-screen 1920x1080 verification screenshots:
   - docs/screenshots/16-email-companion-blade.png (showing the 500x1080 Email Reader/Reply Companion Blade open at x = 1020..1520 next to the 400x1080 Email sidebar at x = 1520..1920 with a real email loaded and the Reply drawer visible)
   - docs/screenshots/16-music-now-playing.png (showing Audio Quality Badge + 4 mode toggles + cover art)
   - docs/screenshots/16-music-queue.png (showing active MPD queue rows & actions)
   - docs/screenshots/16-music-lyrics.png (showing synced lyrics view)
   - docs/screenshots/16-music-playlists-search.png (showing playlists / search view)
   - docs/screenshots/16-notepad-multinote.png (showing multi-note switcher pills + Paste toolbar)
   - docs/screenshots/16-settings-defaults.png (showing default values and active accent icons)
5. Commit and push to origin/main, upload all screenshots and docs/NEVERMORE_COMM_LOG_v17_WORKER.md to Google Drive (Nevermore QC & AI Consultations / 2026-11-05_Phase_13_Complete), and output the final summary report.
