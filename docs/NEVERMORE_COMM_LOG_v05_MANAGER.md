# [MANAGER AI & QC PARTNER DIRECTIVE — `v05_MANAGER` / `v05_QC`]: PHASE 7.5.1 AUDIT FINDINGS → EXECUTE PHASE 7.5.2 (CORRECTNESS FIXES + TEXT-FREE ICON UI + COMPLETE EVIDENCE PASS)

**From:** Manager AI & QC Partner AI (Claude), on behalf of User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Audited Commits:** code `822dde1`, docs/dump `d346757` (`main` HEAD `59970b6`)  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v05_MANAGER.md` (synced to Google Drive `1rPDOAszW8XLARbE4dyGYCNYSm5Izy6Du`)

---

## 0. VERDICT AND OPERATING RULES

1. **Verdict: Keep Phase 8 OFF `main`** (leave any Phase 8 work isolated on `phase-8-plugins`) until **Phase 7.5.2** on `main` passes QC. Most of P0–P3 is genuinely done; what remains is one critical notification parser bug (`image-data` structs & nested arrays), unproven evidence items, remaining main-thread blocking calls, black nav icons (`currentColor` rasterized to `#000000`), and a new immutable UI direction from `bgg`.
2. **New User Decision (Immutable, from `bgg`):** **Remove all visible text that is not user/system data.** No `NEVERMORE // …` header title, no state badge text (`PEEK`/`ENGAGED`/`PINNED`/`HIDDEN`), no `DND OFF`, no `Clear All`, no section header labels, no terminal tab titles. Use a modern, consistent **Lucide SVG icon set** dynamically colored from the active theme palette, with tooltips and accessible labels on every button (see **Part B**).
3. **Real Commit SHAs Only:** Commit code first on `main`, run `git rev-parse --short HEAD`, record that exact SHA in `docs/EVIDENCE_LOG.md` and `NEVERMORE_COMM_LOG_v06_WORKER.md`, then create a clean follow-up docs/dump commit and record **both** SHAs (also backfill `822dde1` / `d346757` / `59970b6` into the 7.5.1 log row).
4. **Pre-Test Notification Protocol 0.2 + Clean Screenshots (No Toast Overlap):** Keep the 5-second `notify-send` warning before live UI/screenshot tests, **then toggle DND ON (or wait for the warning toast to close) before running `spectacle`** so the KDE Plasma notification popup never covers the top-right of the sidebar in your screenshots (see **C2**).
5. **"Verified Beats Claimed" (`AGENTS.md`):** Anything without recorded proof in `docs/EVIDENCE_LOG.md` is **UNVERIFIED**. Replace the single consolidated 7.5.1 row with one row per claim (exact command, raw output, timestamp, screenshot path, and commit SHAs).
6. **Strict Execution Order:** Execute **Part A (Correctness Fixes) → Part B (Text-Free Icon UI) → Part C (Single Comprehensive Evidence Pass)** so all screenshots and pixel measurements are captured once on the final text-free UI.

---

## 1. CONFIRMED RESOLVED IN `822dde1` (DO NOT REGRESS)

- **P0-1 Lifecycle:** `sync_lifecycle` matches spec; `apply_state` is the single source of truth; `last_was_visible` removed.
- **P0-2 Simple Cases:** Immediate emission on flat hints, multi-line body, `desktop-entry` hint, 100-item cap, relative-time refresh in `on_show`.
- **P1-2 & P1-3:** Manifest ID validation, dynamic valid tabs, safe 4-char truncation, `PLUGIN_SDK.md` registration, per-field theme fallback, 16-color VTE guarantee, name/color/font sanitizing, `set_color_cursor`.
- **P2-2 & P2-3:** `PR_SET_PDEATHSIG` + fork-race guard + respawn loop; persistent `gio::DBusConnection` for Inhibit; read-only `ctx.theme()`; `catch_unwind` on lifecycle hooks; notepad `flush_save` on hide; monotonic `next_page_id`.
- **P3:** `.nm-success`/`.nm-warning`, dead `style.css` & `ThemeTokens` deleted, `config.toml` key merge, bundled themes refreshed, docs updated.

---

## PART A — CORRECTNESS FIXES

### A1 (P0): Notification Capture Breaks on Nested Hint Data (`image-data` Structs & Arrays)
**Reproduced by QC** on `crates/nevermore-ui/src/plugins/notifications/parser.rs`:
1. **Nested `]` closes hints early (lines 175–182):** A hint value that is an array (`variant array [ … ]` or `array of bytes [ … ]`) ends with a bare `]`; the parser treats it as the end of the entire `hints` array. Any `desktop-entry` hint after it is ignored!
2. **Any line starting `int32 ` ends the notification (line 183), even inside hints:** `image-data` (sent by Discord, Chrome, Telegram avatars) is a `struct { int32 width; int32 height; int32 rowstride; boolean has_alpha; int32 bits_per_sample; int32 channels; array of bytes [ ... ] }`. Its members print as bare `int32 48` lines! The current parser emits the notification prematurely on the first `int32 48` inside `image-data`, dropping any `desktop-entry` hint that follows it and mis-resolving `discord` instead of `com.discordapp.Discord`.
3. **Multi-line string ending in `"` on an inner line:** A multi-line body whose first line ends in `"` (`He said "hi"` + newline + `and left`) is misread as a single-line string unless lookahead or structural token checking is used.

**Required Implementation (Choose Option 2 First via Short Spike, or Option 1 if Option 2 Fails):**
- **Option 2 (Preferred — Stop Parsing `dbus-monitor` Text):**
  - Spike 07 already approved `BecomeMonitor`. Open a dedicated private `gio::DBusConnection` (`gio::DBusConnection::for_address_sync` to the session bus address) in a background worker, call `org.freedesktop.DBus.Monitoring.BecomeMonitor` with match rule `["type='method_call',interface='org.freedesktop.Notifications',member='Notify'"]`, and register `connection.add_filter(...)`.
  - Extract the `Notify` body directly from `msg.body()` as a typed `glib::Variant` tuple `(String, u32, String, String, String, Vec<String>, HashMap<String, glib::Variant>, i32)` (`susssasa{sv}i`).
  - Then `desktop-entry` is a 100% deterministic dictionary lookup (`hints.get("desktop-entry").and_then(|v| v.str())`), `image-data` structs and multi-line strings with quotes never break anything, and the external `dbus-monitor` subprocess goes away completely!
- **Option 1 (Fallback + Mandatory Hardening of `DbusNotificationParser`):**
  - Whether you use Option 2 for live capture or keep `DbusNotificationParser`, you **must** also fix `DbusNotificationParser` in `parser.rs` so all unit tests pass:
    1. Track bracket/brace/paren nesting depth inside `in_actions` and `in_hints` (increment on lines ending with `[`, `{`, `(`; decrement on lines that are `]`, `}`, `)`). Close `in_actions` or `in_hints` **only** when depth returns to `0`.
    2. Accept `int32 <expire_timeout>` as the end of `Notify` **only** after `in_hints` has closed (`arg_index == 7` and `depth == 0`)!
    3. Handle multi-line strings whose first line ends with `"` by buffering or peeking the next line to verify it is a structural D-Bus token (`uint32 `, `string "`, `array [`, `dict entry(`, `variant `, `int32 `, `]`, `)`, `}`).

**Mandatory Unit Tests in `parser.rs`:**
Test all 5 cases below and assert `emitted.len() == 1`, `desktop_entry == "com.discordapp.Discord"`, and emission on the final `int32 -1`:
1. `image-data` struct **before** `desktop-entry`:
```text
method call time=1793753550.426000 sender=:1.10675 -> destination=org.freedesktop.Notifications serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "discord"
   uint32 0
   string ""
   string "Alice"
   string "hey, are you online?"
   array [
   ]
   array [
      dict entry(
         string "image-data"
         variant             struct {
               int32 48
               int32 48
               int32 192
               boolean true
               int32 8
               int32 4
               array of bytes [
                  ff 00 00 ff ff 00 00 ff ff 00 00 ff
               ]
            }
      )
      dict entry(
         string "desktop-entry"
         variant             string "com.discordapp.Discord"
      )
   ]
   int32 -1
```
2. `image-data` struct **after** `desktop-entry`.
3. Nested variant array `[ string "a" string "b" ]` before `desktop-entry`.
4. Multi-line body where line 1 ends with a quote (`string "He said \"hi\""` / `Line 1 "quoted"\nLine 2`).
5. Existing flat-hint tests.

### A2 (P2): Eliminate All Remaining Main-Thread Blocking Calls
Use `glib::spawn_future_local(async move { let res = gio::spawn_blocking(move || ...).await; ... })` (never holding `RefCell` borrows across `.await`) and enforce a ~2-second timeout on external commands:
- `crates/nevermore-core/src/kwin.rs` (lines 73–104): Add a 2-second timeout helper when invoking `qdbus6` so a hung KWin never blocks forever.
- `crates/nevermore-ui/src/plugins/notifications/pane.rs` (lines 183–189): Offload `kwin.raise_application(&de)` on notification card click to `gio::spawn_blocking`.
- `crates/nevermore-ui/src/plugins/notifications/pane.rs` (lines 246 & 267): Replace `conn.call_sync(..., -1, ...)` in `toggle_dnd` with asynchronous `conn.call(...)` / `call_future` with a 2000 ms timeout so DND never blocks the GTK thread.
- `crates/nevermore-ui/src/plugins/widgets/pane.rs` (lines 240–249 & 427–431): Move `send_mpris_cmd` (`playerctl previous/play-pause/next`) off the GTK thread into `gio::spawn_blocking` / background thread.
- `crates/nevermore-ui/src/main.rs` (lines 652–697): Offload the blocking IPC arms (`ListSessions`, `CreateSession`, `KillSession`, `ListTasks`, `LaunchTask`, `GetTaskLogs`, `StopTask`, `RaiseApp`) onto `gio::spawn_blocking` and send the response onto `cmd.responder` when the future resolves.
- `crates/nevermore-ui/src/plugins/terminal/pane.rs` (~lines 315, 331, 378–410): Move synchronous tmux / systemd subprocess calls (`list_sessions`, `is_session_backgrounded`, `set_session_backgrounded`, `kill_session`, `list_tasks`, `stop_task`) in button handlers and `refresh_background_list` off the GTK main thread where possible (or cache/async-update `refresh_background_list` via `gio::spawn_blocking`).

### A3 (P2): Reap `wpctl` Child Processes in `widgets/pane.rs` (Lines 211–218)
`Command::new("wpctl")...spawn()` drops the `Child` handle without calling `.wait()`, leaving one defunct `<defunct>` zombie process per slider tick! Change `.spawn()` to `.status()` inside the background thread so the child is properly waited and reaped.

### A4 (P2): Fix Lifecycle & Terminal Focus Ordering in `main.rs` (Lines 273–277)
Currently `ctx.registry.borrow_mut().sync_lifecycle(is_visible, &active_tab)` runs before `ctx.panel_window.set_visible(true)` and `set_keyboard_mode(KeyboardMode::OnDemand)`. Because the window is not visible/focusable yet when `on_show()` runs, `terminal.grab_focus()` fails silently!  
**Fix:** Move `ctx.registry.borrow_mut().sync_lifecycle(is_visible, &active_tab)` to run **after** the `match state { ... }` block in `apply_state` (and inside `TerminalPane::grab_terminal_focus()`, also schedule `term.grab_focus()` via `glib::idle_add_local_once` so GTK focuses the VTE widget after the window maps). Prove it in **C5**.

### A5 (P2): Prevent Stale DND State if Notification Daemon Restarts (`notifications/pane.rs`)
Watch `org.freedesktop.Notifications` via `gio::bus_watch_name_on_connection` (or query the `Inhibited` property asynchronously inside `on_show()`) so if `plasmashell` restarts and drops the inhibition cookie, `dnd_cookie` resets to `None` and the DND button icon reflects the true state.

### A6 (P2): True Headless Unit Tests & Working GitHub Actions CI
1. **GTK-Free Lifecycle Unit Test (`plugins/mod.rs`):** Currently `test_sync_lifecycle_state_machine` calls `gtk4::init()` and `ScrolledWindow::new()`, which panics in headless environments without a Wayland/X11 display! Extract the pure state-tracking logic (`LifecycleTracker` or make `PluginEntry` only create a `ScrolledWindow` when mounted, or test `sync_lifecycle` without calling `gtk4::init()` / `ScrolledWindow::new()`) so `cargo test` passes with `DISPLAY="" WAYLAND_DISPLAY=""`.
2. **D-Bus-Free Guard on `systemd::tests::test_systemd_task_launch_and_inspect` (`crates/nevermore-core/src/systemd.rs`):** If `systemctl --user status` or `DBUS_SESSION_BUS_ADDRESS` is unavailable (e.g., in a headless container/CI sandbox), skip the test cleanly with `eprintln!("Skipping systemd user bus test: no user D-Bus session"); return;` instead of failing.
3. **Fix `.github/workflows/ci.yml`:** Install the required Ubuntu system packages before cargo build/test (`sudo apt-get update && sudo apt-get install -y libgtk-4-dev libgtk4-layer-shell-dev libvte-2.91-gtk4-dev pkg-config tmux`) and run `cargo test --workspace --locked` with `DISPLAY="" WAYLAND_DISPLAY=""` to prove headless tests pass.

### A7 (P3): Small Hardening Items
- Wrap `plugin.build_widget(ctx)` inside `std::panic::catch_unwind` in `PluginRegistry::register()`.
- In `theme.rs` (`set_active_theme_name`): if an existing `config.toml` fails to parse as valid TOML, log a warning (`eprintln!`) and back it up to `config.toml.bak` before writing the new table.
- In `icons.rs` / `nav_rail.rs`: support loading a custom relative SVG path from `ctx.config_dir` / plugin resources (rejecting `..`, absolute paths, and files > 64 KB), falling back to the puzzle icon.

---

## PART B — TEXT-FREE UI WITH A CONSISTENT LUCIDE ICON SET (NEW IMMUTABLE USER DECISION)

### B0. Core Design Rule
**Visible text is permitted ONLY when it is live data the user reads:**
- **Allowed Data Text:** notification `app_name` / `summary` / `body` / short relative time (`now`, `5m`, `2h`, `3d`), clock time (`HH:MM:SS`) & date, CPU % / RAM % numbers, notepad user text & compact numeric counts, terminal PTY output & compact page numbers (`1`, `2`, `3`).
- **Everything else MUST be a Lucide SVG icon** with a tooltip (`set_tooltip_text`) and GTK accessible label (`update_property(&[gtk4::accessible::Property::Label(...)])`). **Zero emojis** (`📌`, `✕`, `🔔`, `🔕`, `🔊`, `🔇`, `⏮`, `⏯`, `⏭`, `⏏`) anywhere in the codebase!

### B1. Complete Text Kill-List Table
Replace every static UI label/emoji below with its Lucide icon equivalent:

| Location | Remove This Text / Emoji | Replace With (Lucide Icon + Tooltip) |
| :--- | :--- | :--- |
| `main.rs` header `title_lbl` | `"NEVERMORE // <TAB>"` | Remove widget completely (nav rail active icon + tooltip already identify the tab). |
| `main.rs` header `badge_state` | `"HIDDEN"` / `"PEEK"` / `"ENGAGED"` / `"PINNED"` | Remove widget completely (see B4 for non-text visual state signaling). |
| `main.rs` header pin & close buttons | `"📌"`, `"✕"` | Lucide `pin` (when unpinned) ↔ `pin-off` (when pinned, colored in `accent_fg` on `accent_color` bg); Lucide `x` for close. |
| `notifications/pane.rs` header | `"NOTIFICATIONS FEED"`, count badge, `"🔔 DND OFF"` / `"🔕 DND ON"`, `"Clear All"` | Remove header title. DND button: Lucide `bell` (off) ↔ `bell-off` (on, active accent style). Clear button: Lucide `trash-2`. Move unread/total count to a small numeric badge overlay on the nav-rail bell button (hidden when 0), or compact icon badge. |
| `notifications/pane.rs` empty state | `"No recent notifications"` | Centered dim Lucide `inbox` icon (32px, `text_secondary` color), zero text. |
| `notifications/pane.rs` relative time | `"Just now"`, `"5m ago"` | Shorten to data-only: `"now"`, `"5m"`, `"2h"`, `"3d"`. |
| `terminal/pane.rs` | `"{num}: {session_name}"` page titles, `"+"`, `"⏏"`, `"✕"`, `"BACKGROUND SESSIONS & TASKS"`, `"tmux: ..."`, `"Attach"`, `"Stop"` | Page buttons show compact number chips only (`"1"`, `"2"`, `"3"`, with session name in tooltip). Action buttons: Lucide `plus` (new), `layers` (send to background), `x` (close). Background section: Lucide `layers` icon header (no title text); rows show session/task name (data) + Lucide `corner-up-left` or `play` (reattach) and `x` / `trash-2` (kill/stop). |
| `widgets/pane.rs` | `"SYSTEM RESOURCES"`, `"CPU"`, `"RAM"`, `"AUDIO OUTPUT"`, `"🔊"`/`"🔇"`, `"MEDIA CONTROLLER"`, `"No active player"`, `"Idle"`, `"Unknown Artist"`, `"⏮"`, `"⏯"`, `"⏭"` | Remove all section title labels! CPU row: Lucide `cpu` icon + progress bar + XX%. RAM row: Lucide `memory-stick` (or `hard-drive`) icon + progress bar + XX%. Audio row: Lucide `volume-2` ↔ `volume-x` button + slider. Media card: when idle/no player, show a centered dim Lucide `music` icon (24px) with no `"No active player"` / `"Idle"` words; when playing, show track title & artist (data) + Lucide `skip-back`, `play`/`pause`, `skip-forward` icon buttons. |
| `notepad/pane.rs` | `"SCRATCHPAD"`, `"Clear"`, `"X words \| Y chars"` | Remove `"SCRATCHPAD"` header title; use Lucide `eraser` (or `trash-2`) icon button for clear; footer shows compact numeric data only (e.g. `"12w · 64c"` or icon + numbers, no English words). |
| `placeholders/mod.rs` (media, gemini, youtube, vpn, email) | `"NEVERMORE // ... (Phase 8-10 Tab)"` | One centered dim (`text_secondary`) 40px Lucide icon for that tab (`media.svg`, `gemini.svg`, `youtube.svg`, `vpn.svg`, `email.svg`), zero text! |

### B2. Theme-Aware Dynamic SVG Icon Engine (`crates/nevermore-ui/src/icons.rs`)
**Why all 9 nav rail icons rendered black (`#000000`, 1.29:1 contrast) in 7.5.1:**  
The bundled SVGs use `stroke="currentColor"`, and `gdk::Texture::from_bytes` rasterizes standalone SVG bytes without GTK CSS context—so `currentColor` defaults to `#000000` (black)!

**Required Implementation:**
1. Create `crates/nevermore-ui/src/icons.rs` bundling clean 24x24 `stroke="currentColor"` Lucide SVGs for:
   - The 9 tab icons: `terminal`, `widgets`, `notifications`, `notepad`, `media`, `gemini`, `youtube`, `vpn`, `email`, plus fallback `puzzle`.
   - The UI action/status icons: `pin`, `pin-off`, `x`, `bell`, `bell-off`, `trash-2`, `plus`, `layers`, `cpu`, `memory-stick`, `volume-2`, `volume-x`, `skip-back`, `play`, `pause`, `skip-forward`, `music`, `eraser`, `inbox`, `corner-up-left`.
2. Implement:
   ```rust
   pub fn icon_texture(name: &str, hex_color: &str, size_px: i32) -> gdk::Texture
   pub fn set_button_icon(btn: &gtk4::Button, name: &str, hex_color: &str, size_px: i32)
   ```
   - Before rasterizing, replace `stroke="currentColor"` and any hardcoded `stroke="#..."` in the SVG XML string with `format!("stroke=\"{}\"", hex_color)`, and set `width="{size_px}"` `height="{size_px}"`.
   - Cache textures in a thread-local `HashMap<(String, String, i32), gdk::Texture>`.
3. **High-Contrast Color Rules (≥ 3:1 Contrast Guaranteed):**
   - **Inactive nav/action button icon color:** `palette.text_secondary` (`#a1a1aa` on `#202024` = 5.8:1 contrast; `#a6adc8` on `#181825` in Mocha = 6.5:1 contrast).
   - **Active nav tab button (`.nav-btn.active`):** background is `palette.bg_tertiary`, left border is 3px solid `palette.accent_color`, and the icon color is `palette.accent_color` (`#0a84ff` on `#27272a` = 4.6:1 contrast; `#89b4fa` on `#313244` in Mocha = 6.2:1 contrast).
   - **Primary/Toggled-On button (`.nm-btn-primary`, `.btn-header-pin.pinned`, `.btn-dnd.dnd-active`):** background is `palette.accent_color`, and icon color is `contrast_text_color_for_hex(&palette.accent_color)` (`#11111b` on Mocha `#89b4fa`, `#ffffff` on Nevermore Dark `#0a84ff`).
4. **Live Re-Coloring:** Whenever `on_theme_changed(&palette)` or state changes (`set_active_tab`, pin toggle, DND toggle, mute toggle), update the button icons with the current palette's hex colors!

### B3 & B4. Slim Icon-Only Header & Non-Text State Signaling
1. **Keep the Header Bar (~36px tall) for the Zone Rule Horizontal Swipe:**
   - Do not delete `header` in `main.rs`—keep `header_scroll_ctrl` attached to it so horizontal two-finger swipes on the header bar switch tabs.
   - Inside `header`, place an empty expanding spacer on the left (`hexpand = true`) and the two right-aligned icon buttons (`btn_pin` with `pin`/`pin-off` 16px icon, and `btn_close` with `x` 16px icon, each inside a 32x32 hit target).
2. **State Signaling Without Text (B4):**
   - **Pinned:** `btn_pin` switches icon to `pin-off` with `.pinned` accent background.
   - **Engaged:** `.sidebar-panel` receives class `.state-engaged` which renders a 2px solid `{accent_color}` border on the panel's left edge (`border-left: 2px solid {accent_color};`).
   - **Peek:** `.sidebar-panel` has no accent left border (`border-left: 1px solid {border_color};`), visually signaling that it does not hold keyboard focus.

---

## PART C — SINGLE COMPREHENSIVE EVIDENCE PASS (ALL ROWS IN `docs/EVIDENCE_LOG.md`)

Once Part A and Part B are complete, run one clean verification pass and record individual rows in `docs/EVIDENCE_LOG.md`:
1. **C1 (Exact Commit SHAs):** Backfill `822dde1` / `d346757` / `59970b6` on the 7.5.1 entry, and record both the new Phase 7.5.2 Code Commit SHA and Phase 7.5.2 Docs/Dump Commit SHA.
2. **C2 (DND Proof + Toast-Free Screenshots of All 9 Tabs in Both Themes):**
   - Send the Protocol 0.2 5-second `notify-send` warning and sleep 6 so the warning toast disappears.
   - Programmatically trigger or click the DND toggle ON, then run:
     `busctl --user get-property org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications Inhibited`
     and log the raw output proving `b true`.
   - While DND is ON (guaranteeing zero Plasma toast popups can photobomb the screen!), capture clean screenshots of the tabs in `nevermore-dark` and `catppuccin-mocha` (including `07_5_2-tab-terminal.png`, `07_5_2-tab-widgets.png`, `07_5_2-tab-notifications.png`, `07_5_2-tab-notepad.png`, `07_5_2-tab-placeholders.png`, and `07_5_2-theme-mocha.png`).
   - Toggle DND OFF, re-run `busctl --user get-property ... Inhibited` to prove `b false`, and then send the Protocol 0.2 all-clear notification.
3. **C3 (E1 Hot-Edge Geometry, y = 50 / 540 / 1030 Hit Test, & 1500ms Peek Grace Timer):**
   - Log `hot_edge_window.width()`, `hot_edge_window.height()`, and `hot_edge_box.height()` when mapped in `Hidden` state.
   - Test pointer hover at `x = 1919, y = 50`, `x = 1919, y = 540`, and `x = 1919, y = 1030` (via `ydotool mousemove --absolute -x 1919 -y <Y>` or test harness) and record `nevermore status` transitioning `Hidden -> Peek` at all three vertical spots.
   - Prove the 1500 ms Peek grace timer: enter hot edge without entering the panel, wait 1.8s, and record `nevermore status` returning to `Hidden`.
4. **C4 (True 340px Clamp Proof with 1000px Child):**
   - Add a unit/integration test or `--test-clamp` check that mounts a Label / Box with `set_size_request(1000, 50)` inside `wrap_in_plugin_container`, maps `panel_window`, and logs `panel_window.width() == 400` (left edge `x = 1520`).
5. **C5 (Terminal Focus Proof on `nevermore show terminal`):**
   - From `Hidden`, run `nevermore show terminal`, verify `TerminalPane::grab_terminal_focus()` focuses the active VTE widget (`terminal.has_focus() == true`), send keystrokes, and verify capture in the tmux pane.
6. **C6 (Icon Contrast Verification Script `scripts/verify_icon_contrast.py`):**
   - Commit and run `scripts/verify_icon_contrast.py` on the captured `nevermore-dark` and `catppuccin-mocha` screenshots, measuring the contrast ratio of every nav rail icon (inactive and active) and header/action icon against its button background. Assert and log that every icon achieves ≥ 3.0:1 contrast (target 4.5:1+).
   - Also run `grep -rn 'Label::new\|with_label\|set_label\|set_title' crates/nevermore-ui/src` and `grep -nP '[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}]' -r crates/nevermore-ui/src` (must return 0 emojis) and include the DATA-only string classification table in your report.

---

## 2. MANDATORY GATE & DELIVERABLES FOR `v06_WORKER`

1. `cargo fmt --all -- --check` (0 diffs).
2. `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings).
3. `cargo test --workspace --locked` — must pass both in normal session AND headless (`DISPLAY="" WAYLAND_DISPLAY="" cargo test --workspace --locked`).
4. Push clean commits to `main`, generate `docs/05_FULL_SOURCE_CODE_DUMP_Phase-7_5_2.md`, write `docs/NEVERMORE_COMM_LOG_v06_WORKER.md`, and upload all deliverables + screenshots into a new Google Drive subfolder `Nevermore QC & AI Consultations / 2026-11-04_Phase-7_5_2` (under parent folder `1rPDOAszW8XLARbE4dyGYCNYSm5Izy6Du`).
5. Report both exact commit SHAs (code and docs) and keep Phase 8 work strictly on branch `phase-8-plugins` until QC signs off on `v06_WORKER`.
