# [MANAGER AI DIRECTIVE — `v12_MANAGER`]: WORKSTREAM N (SDK UPGRADE, SETTINGS, GMAIL v1, GEMINI FIX, MPD PLUGIN) & WORKSTREAM L (MUSIC LIBRARY MIGRATION & IMPORT CLI)

**From:** Manager AI & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Target:** `main` branch (`https://github.com/BerKetKet/nevermore-sidebar`)  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v12_MANAGER.md`  

---

## PART 1 — WORKSTREAM N: NEVERMORE SDK UPGRADE, SETTINGS PAGE, GMAIL REST API v1, GEMINI FIX, & MPD MUSIC PLUGIN (6 COMMITS ON main)

### Immutable Rules for Workstream N:
1. Pure Rust + GTK4. No Qt/QML/Electron/WebView. No pop-out windows.
2. 400px sidebar (60px rail + 340px content). Plugin content must never exceed 340px.
3. Static chrome uses Lucide icons + tooltips + accessible labels, with case-by-case section headers (.nm-section-header) and field labels (.nm-field-label) where they add structure, plus live data (.nm-data). Emoji characters are strictly forbidden across all code and UI strings.
4. Solid dark surfaces only; colors strict #rrggbb; no rgba, no transparent, no blur.
5. Hidden means 0.0% idle CPU. Timers only run between on_show and on_hide. Never block the GTK main thread (no execute_blocking, no subprocess wait, no blocking socket/file I/O on it). Children are reaped or guarded with PR_SET_PDEATHSIG (spawned from main thread if long-lived).
6. Dependencies stay pinned with "=". Do NOT add crates to Cargo.toml. Allowed: serde, serde_json, toml, thiserror, clap, libc, gtk4, gtk4-layer-shell, vte4, glib, gio, async-channel. Hashing: glib::Checksum. Time: glib::DateTime. HTTP: the curl binary with secrets/headers passed exclusively over stdin (-K -).
7. The four existing core plugins (terminal, widgets, notifications, notepad) must keep compiling and behaving identically.
8. Pre-Test Notification Protocol 0.2: Before any live UI/screenshot test, send the 5-second notify-send warning, sleep 6 seconds, enable DND Inhibit ON, run the test/spectacle, disable DND, and send the all-clear notification.

---

### COMMIT N1: A1 Spike & A3 Cairo CSS-Tinted Icon Engine + Genuine Lucide SVG Bundle
1. Verify gtk4::cairo re-export and GTK >= 4.16 CSS custom properties.
2. Replace manual pixbuf re-coloring and static IconBinding snapshots in crates/nevermore-ui/src/icons.rs with a CSS-driven gtk4::DrawingArea (nm_icon(Icon, Tone, size)) whose draw func reads widget.color() and masks a cached black-alpha surface per (svg_hash, size, scale).
3. Remove ALL fake icon aliases in icons.rs (where clock mapped to SVG_MEDIA, rotate-cw to SVG_WIDGETS, external-link to SVG_TERMINAL, globe to SVG_VPN, shield-off to SVG_SHIELD). Embed genuine 24x24 stroke="currentColor" Lucide SVG paths for every icon: terminal, widgets, notifications, notepad, media, gemini, youtube, vpn, email, puzzle, pin, pin-off, x, bell, bell-off, trash-2, plus, layers, cpu, memory-stick, volume-2, volume-x, skip-back, play, pause, skip-forward, music, eraser, inbox, corner-up-left, shuffle, repeat, repeat-1, list-music, library, mic-vocal, disc-3, search, save, list-plus, chevron-left, chevron-right, refresh-cw, rotate-cw, plug-zap, settings, external-link, clock, globe, shield, shield-off, copy, send, check, mail, key, circle-help.
4. Support Icon::svg(&'static str) for inline SVGs and auto-embed crates/nevermore-ui/resources/icons/*.svg via build.rs. Unknown icon names render circle-help and log a warning.
5. Expose all ThemePalette fields and font variables (--nm-font-sans, --nm-font-mono, --nm-font-size) as CSS custom properties in ThemePalette::to_gtk_css(), and map Tone::{Primary, Secondary, Muted, Accent, Danger, Success, Warning} to .nm-tone-* CSS classes so icons recolor automatically on theme switches and :hover/:checked/:disabled states without on_theme_changed hooks.

---

### COMMIT N2: A2, A5, A6 SDK, SettingSpec, PluginContext, & UiKit Upgrade
1. Extend PluginSpec in crates/nevermore-ui/src/sdk/ with settings: &'static [SettingSpec] and settings_custom: Option<fn(&PluginContext) -> gtk4::Widget>, where SettingSpec defines key, label, and SettingKind::{Text, Secret, Bool, Int, Choice, Path}.
2. Update crates/nevermore-ui/build.rs to scan src/plugins/*/mod.rs (skipping directories starting with "_"), sort by SPEC.order then id, panic on duplicate IDs in tests, and support runtime disabling via ~/.config/nevermore/config.toml ([plugins] disabled = [...]). Update plugins/_template/mod.rs as the canonical reference.
3. Delete the broken mpv-spawning crates/nevermore-ui/src/plugins/media/ directory, and move crates/nevermore-ui/src/plugins/youtube and crates/nevermore-ui/src/plugins/vpn to _youtube and _vpn so build.rs skips them cleanly.
4. Harden PluginContext:
   - ctx.mailbox::<Msg>(handler: FnMut(Msg, &Tx<Msg>)) -> Tx<Msg> running sequentially on the main loop.
   - Wrap every closure registered through the SDK (mailbox, ctx.every, on_click, on_submit) in std::panic::catch_unwind(AssertUnwindSafe) and log panics.
   - Cmd builder: Cmd::new(prog).arg().args().env().timeout(Duration).stdin_bytes(bytes) and Cmd::curl(url) (--max-time, --proto =https, -fsS, User-Agent). ctx.run(cmd, &tx, map_fn) spawns with process_group(0) + pre_exec prctl(PR_SET_PDEATHSIG, SIGKILL) and kills the whole process group on timeout.
   - Long-lived children (ctx.daemon / ctx.stream) spawn from the main thread (so PDEATHSIG binds to the main thread, not a transient gio pool thread) and reap on a blocking thread.
   - ctx.every(Duration, cb) starts on show (firing once immediately) and stops on hide.
   - ctx.load_json::<T: Default + DeserializeOwned>(name) and ctx.save_json(name, &T) with [a-z0-9._-]+ name validation, per-path sequence numbers, atomic temp-file + fsync + rename on a blocking thread, and .bad quarantine on corrupt JSON.
   - ctx.secret_get(key, &tx, map) and ctx.secret_set(key, val, &tx, map) via secret-tool passing secrets strictly on stdin with attributes service nevermore, plugin <id>, key <k>.
   - ctx.setting_str(key), ctx.setting_bool(key), ctx.setting_int(key) reading from ~/.config/nevermore/plugins/<id>.toml.
5. Upgrade UiKit in crates/nevermore-ui/src/sdk/ui_kit.rs (all 340px-safe):
   - nm_page, nm_card, nm_section_header(text) (.nm-section-header), nm_field_label(text) (.nm-field-label), nm_icon, nm_icon_button, nm_toggle_icon_button (with set_active_silent), nm_data_label(text) (.nm-data, single-line ellipsized), nm_prose_label(text) (.nm-data.nm-prose, multi-line WrapMode::WordChar, selectable, clamped <= 340px), nm_toolbar, nm_status_dot, nm_empty_state, nm_metric_row, nm_list, nm_list_row, nm_search_entry (200ms debounce), nm_prompt_entry(icon, placeholder, on_submit) (.nm-prompt-entry, triggers ONLY on connect_activate / Enter key and clears entry text), and nm_segmented.

---

### COMMIT N3: Built-In Sidebar Settings Page & Pango System Font / Tab Manager
1. In crates/nevermore-ui/src/nav_rail.rs, add a bottom-pinned settings gear icon button (separated by an expanding vertical spacer from the plugin tabs above it) that switches the content stack to the built-in Settings Page ("__settings__").
2. Implement crates/nevermore-ui/src/settings_page.rs (clamped to 340px width):
   - Card 1 (APPEARANCE & THEME): Active theme dropdown selector (listing ~/.config/nevermore/themes/) + OKLab [seed] hex inputs (bg, fg, accent) with live apply.
   - Card 2 (TYPOGRAPHY & SYSTEM FONTS): Enumerate installed system font families via Pango (pango_context().list_families()). Provide a UI Font Family selector, a Monospace Font Family selector, and a Font Size spin/scale (10..18px). Persist to ~/.config/nevermore/config.toml ([appearance]) and apply live to CSS (--nm-font-sans, --nm-font-mono, --nm-font-size) and VTE terminals.
   - Card 3 (NAVIGATION TABS): Toggle switches for each discovered plugin updating [plugins] disabled = [...] in config.toml and showing/hiding nav rail buttons live.
   - Card 4+ (PER-PLUGIN SETTINGS): Automatically render a card for every plugin that declares non-empty SPEC.settings or SPEC.settings_custom, persisting values to ~/.config/nevermore/plugins/<id>.toml (or Secret Service for SettingKind::Secret) and notifying the plugin on change.

---

### COMMIT N4: Gemini Plugin Remediation & Official Gmail REST API v1 Plugin
1. Remediate crates/nevermore-ui/src/plugins/gemini/mod.rs:
   - Never put the API key in the URL or argv. Pass header "x-goog-api-key: <key>" and URL via curl -K - on stdin.
   - Register gemini_api_key (SettingKind::Secret) and model (SettingKind::Choice: ["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"]) in SPEC.settings.
   - Replace nm_search_entry with nm_prompt_entry so prompts only fire on Enter and clear the input box immediately.
   - Render conversation turns using multi-line wrapped nm_prose_label inside nm_card (remove the 57-byte slice!), with a copy-to-clipboard icon button on each assistant turn and a clear-history button in the toolbar.
2. Rewrite crates/nevermore-ui/src/plugins/email/mod.rs for Official Gmail REST API v1 ONLY:
   - Purge all references to kmail and himalaya from the entire codebase and docs.
   - Register client_id (Text), client_secret (Secret), and poll_interval_sec (Int, default 120) in SPEC.settings, plus a settings_custom "Authorize Gmail" button.
   - Clicking "Authorize Gmail" binds an ephemeral std::net::TcpListener on 127.0.0.1:0 on a background thread, opens the Google OAuth2 consent URL (scope https://www.googleapis.com/auth/gmail.modify) via xdg-open, captures the loopback redirect code, exchanges it via https://oauth2.googleapis.com/token over curl stdin, and stores gmail_refresh_token in Secret Service. Automatically refresh access tokens using gmail_refresh_token.
   - Fetch unread inbox messages via https://gmail.googleapis.com/gmail/v1/users/me/messages?q=is:unread+in:inbox&maxResults=15 (passing Authorization: Bearer <token> via curl -K - stdin).
   - Display each unread email with Sender, Subject (must be visible!), and Timestamp/Snippet, plus two icon actions: Open Thread in Browser (xdg-open https://mail.google.com/mail/u/0/#inbox/<threadId>) and Mark as Read (POST /modify removing UNREAD).

---

### COMMIT N5: B1 sdk::mpd Module & B2/B3/B5/B6/B7 MPD Music Plugin Groundwork
1. Implement crates/nevermore-ui/src/sdk/mpd.rs:
   - Pure-Rust std::os::unix::net::UnixStream (~/.config/mpd/socket, fallback $XDG_RUNTIME_DIR/mpd/socket) and std::net::TcpStream (127.0.0.1:6600) transport.
   - Verify greeting "OK MPD <version>", parse "key: value" lines ending in "OK" or "ACK [code@idx] {cmd} msg", quote args escaping \ and " and rejecting newlines, support command_list_begin/end, and parse binary responses ("size: N", "binary: N", N raw bytes, newline, "OK") for readpicture and albumart.
   - Enforce 2s read/write timeouts, 64KB line cap, 8MB binary cap, typed Status and Song structs, and a visible-only idle thread ("idle player playlist mixer options stored_playlist database update") that forwards events via async_channel and shuts down cleanly on hide via shutdown(Both) on a cloned stream + join.
   - Write unit tests (arg escaping, response/binary parser) plus a fake MPD server test harness using UnixListener in a background thread.
2. Implement crates/nevermore-ui/src/plugins/music/mod.rs (id = "music", order = 5, [NOW] Now-Playing view):
   - Register socket, host, port, and music_dir in SPEC.settings.
   - Render 260px square album cover art (trying readpicture <uri> <offset> first, falling back to albumart <uri> <offset>, cached in a 32-entry LRU by album directory, showing disc-3 empty state when no cover exists).
   - Render ellipsized Title, Artist, and Album nm_data_labels, seek slider with elapsed/total nm_data_labels (interpolated monotonically every 1s only while visible and playing), prev / play-pause toggle / next icon buttons, and a wired volume slider + mute toggle calling MPD setvol.
   - When MPD is unreachable, show nm_empty_state("plug-zap", Tone::Warning) and retry with capped backoff only while visible. Never touch ~/Music files or spawn external players.

---

### COMMIT N6: A4 Contract Test Hardening & Workstream N Verification
1. Update crates/nevermore-ui/tests/plugin_contract.rs:
   - Never silently pass when no display is present (require WAYLAND_DISPLAY or fail loudly).
   - Install a GLib log writer/handler that fails on any Gtk-CRITICAL or Gtk-WARNING.
   - Walk every built plugin widget tree: require all non-empty Labels to have .nm-data, .nm-section-header, or .nm-field-label; require Buttons to have empty text labels; ban emojis (U+1F300..1FAFF, U+2600..27BF) in widget trees and plugin source files.
   - Assert horizontal minimum width <= 340 inside the shell ScrolledWindow wrapper.
   - Run build -> show -> pump 100ms -> hide -> show -> hide -> on_theme_changed under catch_unwind and assert 0 active timers in the PluginContext registry after hide.
   - Enforce source lints banning raw Command::new, timeout_add, execute_blocking, and literal hex/rgba in plugin CSS.
2. Run cargo test --workspace --locked and WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract.

---

## PART 2 — WORKSTREAM L: MUSIC LIBRARY RESTRUCTURE, MIGRATION, & STANDALONE IMPORT CLI (L0–L7 + N8 INTEGRATION)

1. L0 Preflight & Backup Safety Check:
   - Check df -h ~/Music, findmnt -T ~/Music -o FSTYPE,SOURCE, and verify stat -c %d ~/Music and stat -c %d /home/bgg/Music/MusicBee return the exact same device ID (confirming atomic rename(2) on the same filesystem). If they are NOT on the same filesystem, STOP Workstream L and report.
   - Check if the filesystem supports reflinks (btrfs/xfs); if supported, create a reflink safety snapshot cp -a --reflink=always /home/bgg/Music/MusicBee /home/bgg/Archive/MusicBee-reflink-backup before L4.
   - Inspect ~/.config/mpd/mpd.conf and verify python-mutagen (or ffprobe) is available for tag reading.

2. L1 Read-Only Enumeration:
   - Scan /home/bgg/Music/MusicBee/ without modifying anything and generate docs/music_migration/L1_INVENTORY_REPORT.md covering: counts and byte totals by extension, audio formats, files missing artist/album/title/track tags, folder-vs-tag disagreements, multi-disc albums, compilations, duplicate candidates (by tag tuple and exact SHA-256 byte hash), non-audio files (cover images, .cue, .log, .lrc, .m3u, .mbp, .txt), NTFS-illegal chars or trailing dots/spaces, non-Latin scripts (Ge'ez/Amharic, CJK, Arabic, etc.), path components >255 bytes, #lyrics mapping/format, and Playlists (.m3u/.m3u8 vs .mbp).

3. Build the Shared Migration & Import Tool (scripts/music-import installed to ~/.local/bin/music-import):
   - Write a clean, reusable standalone CLI script (chmod +x, installed to /home/bgg/.local/bin/music-import and tracked at scripts/music-import, with zero grim/wiz dependencies) implementing the exact L2 target layout:
     * Library root: /home/bgg/Music
     * Track path: ~/Music/<Album Artist>/<Year> - <Album>/<NN> - <Title>.<ext> (multi-disc: <D>-<NN> - <Title>.<ext>; compilations -> "Various Artists"; missing year omits "<Year> - ").
     * Sanitizer: Unicode NFC normalization; preserve all non-Latin scripts (Ge'ez, CJK, Arabic, etc.); replace / \ : * ? " < > | and control characters; strip trailing dots and spaces; enforce 255-BYTE max per path component truncating strictly on UTF-8 character boundaries.
     * Cover art & sidecars: rename album folder images (cover/folder/front.{jpg,png}) to cover.jpg or cover.png; move matching .lrc and .cue sidecars alongside the track's new stem.
     * Non-music routing: Audiobooks -> ~/Audio/Audiobooks; Podcasts -> ~/Audio/Podcasts; Skins, Artist Pictures, MusicBee database files (.mbl, .bak, .lyrics, .pfidx, .ini, .mbp) -> ~/Archive/MusicBee-legacy/.
     * Quarantine (never overwrite, never delete): missing required tags -> ~/Music-Inbox/needs-tags/; path collisions with different SHA-256 -> ~/Music-Inbox/conflicts/; exact byte duplicates -> keep one in ~/Music and quarantine extras to ~/Music-Inbox/duplicates/.
   - Support CLI flags for ongoing use (L7):
     * music-import [--dry-run] [--move] <album_dirs_or_files...>: default copies and verifies SHA-256 hashes (leaving source untouched unless --move is passed), never writes tags, and runs mpc update <relative_path> after import.
     * music-import --migrate-musicbee [--dry-run | --apply | --undo <manifest.tsv>]: executes the L3/L4 migration and undo.
   - Test music-import (both --migrate-musicbee, --undo, and standard album import) on a scratch test directory in /tmp/nevermore_music_test first and record the test output.

4. Execute L3–L6 Migration, Playlists/Lyrics Conversion, & MPD Switch:
   - Generate docs/music_migration/manifest.tsv, docs/music_migration/undo_migration.sh, and docs/music_migration/L3_DRY_RUN_SUMMARY.md (including 20 diverse sample mappings with non-Latin and multi-disc examples).
   - Execute L4 journaled atomic migration: first atomically rename /home/bgg/Music/MusicBee to /home/bgg/MusicBee-legacy, then execute each rename recorded in journal.jsonl into /home/bgg/Music, ~/Audio/{Audiobooks,Podcasts}, ~/Archive/MusicBee-legacy, and ~/Music-Inbox/{needs-tags,conflicts,duplicates}.
   - Verify post-migration integrity: confirm total file counts and byte sums across ~/Music + ~/Audio + ~/Archive/MusicBee-legacy + ~/Music-Inbox equal the L1 inventory 100%, and spot-check SHA-256 hashes on 20 random files. Clean up only truly empty directories inside ~/MusicBee-legacy.
   - Execute L5: convert .m3u/.m3u8 playlists into ~/.config/mpd/playlists/ using the old->new manifest mapping with paths relative to ~/Music (and list proprietary .mbp files in the report for recreation in Euphonica). Copy unambiguous #lyrics files next to their migrated tracks as <track stem>.lrc (or .txt if plain text without timestamps), leaving originals untouched.
   - Execute L6: ensure ~/.config/mpd/mpd.conf sets music_directory "~/Music" with Unix socket ~/.config/mpd/socket and TCP 127.0.0.1:6600 and PipeWire audio output. Restart mpd (systemctl --user restart mpd), run mpc update --wait, and verify mpc stats song count matches the migrated library count. Also verify Euphonica is configured to connect to 127.0.0.1:6600.

5. Execute N8 Live MPD Integration Test, 60s Hidden-CPU Proof, & Final Report:
   - Play a track from the migrated ~/Music library in MPD, open the Nevermore music tab, verify live cover art (readpicture / albumart), metadata, seek bar, and volume control, then run the B7 QA checks (kill/restart mpd recovery, rapid next/prev, non-Latin title rendering, 50x hide/show nlwp check).
   - Hide the sidebar while MPD continues playing for 60 seconds and record ps -o %cpu,rss,nlwp -p $(pgrep -x nevermore-ui) proving 0.0% idle CPU.
   - Capture toast-free screenshots (under Protocol 0.2 + DND lock) of the Settings Page (12-settings-page.png), MPD Music Now-Playing tab with live album art (12-tab-music.png), Gmail tab (12-tab-gmail.png), and Gemini tab (12-tab-gemini.png).
   - Deploy the final release binaries to nevermore.service, commit and push all code and docs to origin/main, upload the complete audit bundle to Google Drive (Nevermore QC & AI Consultations / 2026-11-05_Workstreams_N_and_L_Complete), and write the final report to docs/NEVERMORE_COMM_LOG_v13_WORKER.md.
