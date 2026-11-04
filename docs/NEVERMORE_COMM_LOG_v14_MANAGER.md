# [MANAGER AI DIRECTIVE — `v14_MANAGER`]: FULL 1080p VERTICAL HEIGHT, LAYER::OVERLAY, LIVE SETTINGS PERSISTENCE, MPD/GEMINI/GMAIL LIVE TESTING & RELEASE PASS

**From:** Manager AI, QC Partner AI, & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Base Commit on `main`:** `7f4eb80`  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v14_MANAGER.md`  

---

## DIRECTIVE STATEMENT & TASKS

Execute all 6 tasks below on `main`, committing each fix cleanly and recording empirical outputs and screenshots in `docs/EVIDENCE_LOG.md`:

### 1. FIX 1 — FULL 1080p VERTICAL HEIGHT & LAYER::OVERLAY RIGHT-EDGE PRIORITY (crates/nevermore-ui/src/main.rs):
- In `crates/nevermore-ui/src/main.rs`, `panel_window` and `hot_edge_window` are cutting off at ~half screen height because `set_size_request(..., 0)` and `set_default_size(..., 0)` combined with `panel_window.set_resizable(false)` and `main_box` without `vexpand(true)` cause GTK4 to clamp the surface height to the natural height of `main_box` instead of stretching to the Top + Bottom layer-shell anchors.
- Per explicit user decision, set BOTH `hot_edge_window` and `panel_window` to `Layer::Overlay` (`hot_edge_window.set_layer(Layer::Overlay)` and `panel_window.set_layer(Layer::Overlay)`) with `set_exclusive_zone(-1)`, anchored to `Edge::Right`, `Edge::Top`, and `Edge::Bottom` with 0 margins, so the sidebar displays over anything when the pointer is at the right edge.
- Query the primary monitor height:
  ```rust
  let monitor_height = primary_monitor.as_ref().map(|m| m.geometry().height()).unwrap_or(1080);
  ```
- Explicitly set full `monitor_height` on all top-level containers:
  * `hot_edge_window.set_size_request(HOT_EDGE_WIDTH, monitor_height);`
  * `hot_edge_window.set_default_size(HOT_EDGE_WIDTH, monitor_height);`
  * `hot_edge_box.set_size_request(HOT_EDGE_WIDTH, monitor_height);`
  * `panel_window.set_size_request(PANEL_WIDTH, monitor_height);`
  * `panel_window.set_default_size(PANEL_WIDTH, monitor_height);`
  * `main_box.set_size_request(PANEL_WIDTH, monitor_height);`
  * `main_box.set_vexpand(true);`
  * `nav_rail.widget().set_vexpand(true);`
  * `content_area.set_vexpand(true);`
- Verify after mapping on Wayland that `panel_window.height() == 1080` and `hot_edge_window.height() == 1080`, and verify in a full-screen 1920x1080 spectacle screenshot that the sidebar spans from `y = 0` all the way to `y = 1079` without cutting off mid-screen.

### 2. FIX 2 — LIVE SETTINGS RELOAD, STARTUP PERSISTENCE, & SLIDER FEEDBACK GUARDS:
- **Startup Config Persistence (`main.rs` & `theme.rs`):** On startup (and across theme changes), load saved `[appearance]` overrides (`ui_font`, `mono_font`, `mono_font_size`) and `[plugins] disabled = [...]` from `~/.config/nevermore/config.toml` and apply them immediately to `theme_cell`, `CssProvider`, and `nav_rail` button visibility.
- **Header Icon Recoloring (`main.rs`):** In `on_theme_updated` and `on_appearance_updated`, also update `btn_pin` and `btn_close` icon colors (or convert them to Cairo `TintedIcon` widgets) so theme switches from the Settings Page recolor the header buttons immediately.
- **NavRail Gear Button Highlight (`nav_rail.rs`):** When `active_tab == "__settings__"`, highlight the bottom gear button with the `.active` class and clear `.active` from plugin tab buttons; also exclude `"__settings__"` from horizontal swipe cycling if disabled or keep swipe cycling smooth.
- **Live Plugin Settings Reload (`settings_page.rs`, `gemini`, `email`, `music`):** Ensure that modifying any `SettingSpec` value or Secret on the Settings Page immediately notifies/reloads the target plugin, AND ensure `gemini`, `email`, and `music` re-read `ctx.setting_str` / `setting_int` and `ctx.secret_get` on every `on_show` tick and action so settings changes take effect immediately without restarting `nevermore-ui`.
- **Seek & Volume Slider Suppression Guard (`crates/nevermore-ui/src/plugins/music/mod.rs`):** Guard programmatic `scale.set_value(...)` updates on the seek bar and volume slider with an `is_syncing: Rc<Cell<bool>>` flag so 1-second progress ticks and MPD mixer events never trigger feedback `seekcur` or `setvol` commands back to MPD.

### 3. TASK 3 — CONFIGURE MPD SETTINGS IN NEVERMORE SETTINGS & VERIFY LIVE PLAYBACK:
- Populate `~/.config/nevermore/plugins/music.toml` with:
  ```toml
  socket = "/home/bgg/.config/mpd/socket"
  host = "127.0.0.1"
  port = 6600
  music_dir = "/home/bgg/Music"
  ```
- Verify that opening the Settings Page in `nevermore-ui` displays these exact values in the Music plugin settings card, and verify that the Music tab connects to MPD over `/home/bgg/.config/mpd/socket`, displays the current song's cover art and metadata from `/home/bgg/Music`, and controls playback and volume smoothly.

### 4. TASK 4 — EXTRACT GEMINI API KEY FROM BROWSER, POPULATE SETTINGS, & TEST LIVE CHAT:
- Using your background browser DevTools inspection (or checking Google AI Studio `https://aistudio.google.com/app/apikey` / Google Cloud Console in the logged-in browser profile), retrieve bgg's Gemini API key.
- Store the key via `secret-tool` (`service nevermore, plugin gemini, key gemini_api_key`) and verify the Settings Page shows the Gemini key configured and model set to `"gemini-2.5-flash"`.
- Open the Gemini tab, submit a live test prompt ("Reply with a short 2-sentence system status confirmation for Nevermore Sidebar."), verify the HTTP 200 response from `generativelanguage.googleapis.com` renders inside a multi-line wrapped `nm_prose_label` card without truncation, and capture a screenshot (`docs/screenshots/14-live-gemini-verified.png`).

### 5. TASK 5 — EXTRACT/CONFIGURE GMAIL OAUTH2 CREDENTIALS FROM BROWSER, COMPLETE OAUTH FLOW, & TEST LIVE INBOX:
- Using the logged-in browser session, inspect Google Cloud Console (`https://console.cloud.google.com/apis/credentials`) to locate or create an OAuth 2.0 Desktop Client ID and Client Secret (with Gmail API enabled on the project).
- Populate `client_id` into `~/.config/nevermore/plugins/email.toml` and store `client_secret` into Secret Service (`service nevermore, plugin email, key client_secret`).
- Trigger the "Authorize Gmail" loopback OAuth 2.0 flow, complete the consent click in the browser so `gmail_refresh_token` is saved in Secret Service, and trigger an inbox refresh in the Gmail tab.
- Verify that real unread emails from bgg's Gmail inbox render in the email tab showing Sender, Subject, and Timestamp/Snippet, and capture a screenshot (`docs/screenshots/14-live-gmail-verified.png`, redacting/blurring any sensitive personal email text if needed or verifying layout and pixel geometry).

### 6. TASK 6 — DEPLOY, CAPTURE FULL-HEIGHT SCREENSHOTS, & LOG EVIDENCE:
- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, and `WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract`.
- Build release binaries (`cargo build --release --workspace --locked`), deploy to `/home/bgg/.local/bin/`, and restart `nevermore.service`.
- Using Protocol 0.2 (5s `notify-send` warning -> sleep 6s -> DND Inhibit ON -> `spectacle` -> DND OFF -> all-clear), capture full-screen 1920x1080 screenshots proving the sidebar spans 100% of the vertical height (`y = 0..1080`) with zero cutoff on the Music tab (`14-full-height-music.png`), Gemini tab (`14-live-gemini-verified.png`), Gmail tab (`14-live-gmail-verified.png`), and Settings page (`14-full-height-settings.png`).
- Commit and push to `origin/main`, upload the new screenshots and report to Google Drive, and write the completion report to `docs/NEVERMORE_COMM_LOG_v15_WORKER.md`.
