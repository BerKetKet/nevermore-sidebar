# [MANAGER AI DIRECTIVE — `v03_MANAGER`]: HOLD PHASE 8 ON `main` — EXECUTE PHASE 7.5.1 QC REMEDIATION PASS (P0–P3)

**From:** Manager AI, User (`bgg`), & QC Partner AI (`v03_QC`)  
**To:** Worker AI (Antigravity)  
**Audited Commit:** `main @ 2a19576` (`https://github.com/BerKetKet/nevermore-sidebar`)  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v03_MANAGER.md` (and synced to Google Drive `Nevermore QC & AI Consultations` `1rPDOAszW8XLARbE4dyGYCNYSm5Izy6Du`)

---

## 0. AUDIT VERDICT & OPERATING RULES

1. **Verdict on `main @ 2a19576`:** **HOLD PHASE 8 ON `main`.** The QC Partner AI (`v03_QC`) audited `2a19576` and uncovered critical P0/P1 defects in plugin lifecycle tracking, notification parsing, width clamping, and theme validation. You must fix and empirically prove **all P0–P3 items below** in **Phase 7.5.1** on `main` first.
2. **Never Use Placeholder Commit SHAs:** In `v02_WORKER`, you wrote `main @ [Phase 7.5 release]` instead of the real commit hash (`2a19576`). You must always commit your code *first*, run `git rev-parse --short HEAD`, and record the **actual commit SHA** in your report and `docs/EVIDENCE_LOG.md` (committing the final log update in a clean follow-up commit and recording both SHAs).
3. **Keep Pre-Test Notification Protocol 0.2 Active:** Always send the 5-second `notify-send` warning before any live UI/hover/focus/screenshot test and the all-clear notification afterward.
4. **Autonomous Continuation Rule (Branch Isolation):** Once Phase 7.5.1 is 100% verified, committed to `main`, pushed to GitHub, and uploaded to Google Drive (`NEVERMORE_COMM_LOG_v04_WORKER.md`), if `bgg` is away, do **not** push Phase 8 code onto `main`. Instead, keep `main` locked at the clean Phase 7.5.1 commit for QC audit, and if you continue working while `bgg` is away, create an isolated branch `git checkout -b phase-8-plugins` and record any provisional decisions for user sign-off.

---

## 1. PRIORITY P0 FIXES (CRITICAL LIFECYCLE & NOTIFICATION DEFECTS)

### P0-1 (L1–L4): Make `PluginRegistry` the Single Authoritative Lifecycle State Machine
- **Bugs Found in Audit:**
  - **L1:** `nav_rail` click and header swipe never update `title_lbl`, and `nav_rail` click only updates its private `RefCell` without moving the `.active` CSS class properly via single-sourced state.
  - **L2:** Running `nevermore show <tab>` via CLI while already visible never calls `on_tab_changed`.
  - **L3:** `PluginRegistry::on_tab_changed(prev, new)` uses `if id == prev { on_hide() } else if id == new { on_show() }`. When `prev == new` (user clicks the already-active tab button on the nav rail!), it calls `on_hide()` and **never** calls `on_show()`—killing `Widgets`' 2s timer while visible so the clock freezes!
  - **L4:** `PluginRegistry` is stateless, and `apply_state` calls `on_hide(&current_tab)`. If the user is visible on `widgets`, runs `nevermore show terminal` (L2: no hooks fired), and then hides the sidebar, `on_hide("terminal")` runs while `widgets`' 2s timer keeps running **forever while hidden**, violating the Zero CPU Drain rule!
- **Required Fix:**
  1. Add `shown_tab: Option<String>` to `PluginRegistry` (tracking the exact plugin ID that currently has an active `on_show` state, or `None` when the sidebar is hidden).
  2. Replace `on_show`, `on_hide`, and `on_tab_changed` on `PluginRegistry` with a **single idempotent method**:
     ```rust
     pub fn sync_lifecycle(&mut self, is_visible: bool, active_tab: &str)
     ```
     - Target state is `target: Option<&str> = if is_visible { Some(active_tab) } else { None }`.
     - If `self.shown_tab.as_deref() == target`, do **nothing** (early return—completely eliminates L3!).
     - If `let Some(old_id) = self.shown_tab.take()`, call `on_hide()` on `old_id`.
     - If `let Some(new_id) = target`, call `on_show()` on `new_id` and set `self.shown_tab = Some(new_id.to_string())`.
  3. Delete `last_was_visible` from `UiContext` in `main.rs`. Make `apply_state(ctx: &UiContext)` the **single source of truth** for:
     - Updating `title_lbl` (`NEVERMORE // <UPPERCASE_TAB>`).
     - Updating `nav_rail.set_active(active_tab)` (which must move the `.active` CSS class across buttons).
     - Switching `content_stack.set_visible_child_name(active_tab)`.
     - Calling `ctx.registry.borrow_mut().sync_lifecycle(is_visible, &active_tab)`.
  4. Route **all** tab/state changes (`nav_rail` click, `nav_rail` swipe, `header` swipe, IPC `Show`/`Toggle`/`Hide`/`Pin`, Escape key, close button, hot-edge hover, and leave timer) through `state_machine` + `apply_state(&ctx)`.
  5. **Mandatory Unit Tests in `nevermore-ui`:** Add headless lifecycle unit tests verifying:
     - `Hidden -> Show("widgets")` calls `widgets.on_show()` once.
     - Re-clicking `"widgets"` while visible (`sync_lifecycle(true, "widgets")`) is a no-op (`on_hide` is NOT called).
     - `Show("widgets") -> Show("terminal") -> Hide` calls `widgets.on_hide()`, `terminal.on_show()`, and `terminal.on_hide()` (leaving `shown_tab == None` and zero active timers).

### P0-2 (N1 & N2): Fix the Notification Parser (`crates/nevermore-ui/src/plugins/notifications/pane.rs`)
- **Bugs Found in Audit:**
  - **N1 (High Severity):** The `dbus-monitor` parser only emits a notification when the **next** `method call` header arrives! Repro: send a single `notify-send "Test" "Body"`; it never shows up in the UI until a *second* notification is sent!
  - **N2 (Parser Fragility & Hint Ignored):** Multi-line notification bodies or strings failing `starts_with('"') && ends_with('"')` shift the `current_strings[0..3]` indices. Furthermore, `desktop_entry` is guessed by lowercasing `app_name` instead of parsing the standard FreeDesktop `desktop-entry` variant inside the `a{sv}` hints dictionary (arg #6 of `org.freedesktop.Notifications.Notify`).
- **Required Fix:**
  1. Parse the structure of `org.freedesktop.Notifications.Notify(app_name, replaces_id, app_icon, summary, body, actions, hints, expire_timeout)` deterministically and emit the notification **immediately** as soon as the closing `int32 <expire_timeout>` line of the `Notify` call is read—never waiting for a subsequent `method call`!
  2. Properly handle multi-line strings in `summary` / `body` without corrupting field indices, and extract the `"desktop-entry"` string from the `dict entry` inside the `hints` array (falling back to `app_name` only if the `"desktop-entry"` hint is absent).
  3. Cap the in-memory notification list and UI `ListBox` at a maximum of `100` items (evicting the oldest when exceeded so memory never grows unbounded), and format timestamps cleanly (or update relative time labels on `on_show()`).
  4. **Mandatory Unit Test & Live Proof:** Write unit tests in `notifications` feeding sample `dbus-monitor` stdout (including a single notification with a multi-line body and a `"desktop-entry"` hint) into the parser function and asserting immediate emission. Also verify live with a single `notify-send` command.

---

## 2. PRIORITY P1 FIXES (REAL WIDTH CLAMP, REGISTRY-DRIVEN TABS, & THEME VALIDATION)

### P1-1 (3.1): Enforce a True 340px Maximum Width Clamp on Plugin Widgets
- **Bug Found in Audit:** `wrapper.set_size_request(340, -1)` only sets the *minimum* width in GTK4, and `Overflow::Hidden` only clips rendering without preventing child widgets from requesting a larger natural width and pushing the layer-shell window wider than `400px`. Thus `docs/PLUGIN_SDK.md`'s claim that a plugin *"can never stretch the sidebar wider than 400px"* is currently false.
- **Required Fix (`crates/nevermore-ui/src/plugins/mod.rs`):**
  - Wrap every plugin widget inside a `gtk4::ScrolledWindow` (or custom clamping container) with:
    - `hscrollbar_policy = gtk4::PolicyType::Never`
    - `vscrollbar_policy = gtk4::PolicyType::Automatic` (or `Never` for plugins managing their own vertical scroll like Terminal)
    - `propagate_natural_width = false`
    - `min_content_width = 340` and `max_content_width = 340`
    - `set_size_request(340, -1)` and `set_hexpand(false)`
  - Verify with a test widget containing a `1000px`-wide non-wrapping child that `panel_window.width()` remains strictly `400px`. Update `docs/PLUGIN_SDK.md` to accurately describe this container.

### P1-2 (3.3): Make Valid Tabs Registry-Driven & Fix End-to-End `PLUGIN_SDK.md`
- **Bugs Found in Audit:**
  - `CANONICAL_TABS` / `validate_tab` in `nevermore-core` hard-gates tab IDs to the original 9 tabs. If a user follows `docs/PLUGIN_SDK.md` to add `HelloPlugin` (`id = "hello"`), `nav_rail` shows it, but `SidebarStateMachine::set_active_tab("hello")` and `nevermore show hello` reject it with an error and snap the UI back to the old tab!
  - `nav_rail.rs` hardcodes SVG icons by ID (`match id.as_str()`) and ignores `manifest.icon`. It also slices `&id[..4]` for fallback labels, which panics on 1–3 char IDs or non-ASCII strings!
  - `docs/PLUGIN_SDK.md` omits `pub mod hello;` and requires two lines instead of the single registration call.
- **Required Fix:**
  1. In `nevermore-core`:
     - Validate `PluginManifest::id` at registration/parse time: must match `^[a-z0-9_-]{1,32}$` and reject duplicate plugin IDs or path traversal (`..`, `/`).
     - Allow `SidebarStateMachine` to be initialized with the dynamic set of registered plugin IDs (defaulting to the 9 standard tab IDs when constructed without a custom list, and updated from `PluginRegistry::plugin_ids()`), so any valid registered plugin works seamlessly with `set_active_tab` and `nevermore show <id>`!
  2. In `nav_rail.rs`:
     - Load the icon from `manifest.icon` (looking up bundled SVG icons by filename/name or loading an SVG path from resources/plugin directory, with a safe generic puzzle-piece fallback SVG if not found).
     - Replace `&id[..4]` with safe char truncation (`id.chars().take(4).collect::<String>()`).
  3. Update `docs/PLUGIN_SDK.md` so the registration example is 100% accurate and works end-to-end (including `nevermore show hello`).

### P1-3 (T1, T2, T3, T9): Harden `ThemePalette` Validation, Per-Field Fallback, & VTE Palette Safety
- **Bugs Found in Audit:**
  - **T1:** `terminal/pane.rs` uses `filter_map(|c| gdk::RGBA::parse(c).ok())`. If a theme has 1 invalid hex in `term_palette`, `palette_rgba` has 15 items; `vte4::Terminal::set_colors` rejects any palette length other than 0/8/16/232/256 and leaves the terminal stuck on old colors!
  - **T2:** Color strings and font names are interpolated raw into CSS (allowing `rgba()` / `transparent` violations of the solid-color rule, CSS injection via `"` or `*/`, and path traversal in `load_theme("../x")`).
  - **T3:** `#[serde(default)]` fails the entire TOML file if a single field has the wrong type or `term_palette` has 15/17 items, and startup silently calls `unwrap_or_default()` without logging why the user's theme failed.
- **Required Fix (`crates/nevermore-core/src/theme.rs` & `terminal/pane.rs`):**
  1. **Theme Name Validation:** Restrict theme names in `load_theme(name)` and `set_active_theme_name(name)` strictly to `^[a-zA-Z0-9_-]{1,64}$` (rejecting `/`, `\`, `..` immediately).
  2. **Per-Field Validation & Fallback (`ThemePalette::from_toml_str_lossy`):**
     - Parse TOML into a `toml::Value` table first. For every field, validate its format and fall back **per-field** to `ThemePalette::default()` (Nevermore Dark) with a clear `eprintln!("[nevermore-theme] warning: ...")` log if a field is invalid or missing:
       - All color fields (`bg_primary`, ..., `term_bg`, `term_fg`, `term_cursor`, and each of the 16 `term_palette` slots): must strictly match solid 6-digit hex `^#[0-9a-fA-F]{6}$` (rejecting `rgba`, `transparent`, 3/8-digit alpha hex, or CSS injection). If `term_palette` does not have 16 valid `#rrggbb` entries, fall back per-index (or to the 16 default ANSI colors) so `term_palette` is **guaranteed** to always contain 16 valid `#rrggbb` strings.
       - `ui_font` and `mono_font`: strip/reject `"` `;` `{` `}` `\` `/*` `*/` characters.
       - `name`: strip `*/` and newlines before placing in CSS comments.
  3. **Apply `term_cursor` in VTE (T4):** In `terminal/pane.rs`, call `term.set_color_cursor(gdk::RGBA::parse(&palette.term_cursor).ok().as_ref())` in addition to `set_colors`.
  4. **Mandatory Unit Tests (T9):** Add unit tests in `nevermore-core/src/theme.rs` testing:
     - Malformed TOML and wrong field types falling back cleanly with warnings.
     - `term_palette` with 15 items or invalid hex strings still producing 16 valid `#rrggbb` colors.
     - Rejection of `rgba(...)`, `transparent`, CSS injection strings, and `../` theme names.

---

## 3. PRIORITY P2 FIXES (HOT-EDGE PROOF, SUBPROCESS REAPING, DND, READ-ONLY CONTEXT, & NON-BLOCKING UI)

### P2-1 (E1–E4): Hot-Edge Input Region Proof, Dynamic Monitor Sizing, & Peek Grace Timer
1. **E1 (Missing Proof):** Programmatically query and log `hot_edge_window.height()`, `hot_edge_window.width()`, and `hot_edge_box.height()` while in `SidebarState::Hidden`, and test simulated/real pointer motion or hit testing at `y = 50`, `y = 540`, and `y = 1030` in `docs/EVIDENCE_LOG.md`.
2. **E2 & E3 (Monitor Binding & Dynamic Height):**
   - Bind both `hot_edge_window` and `panel_window` to the same primary `gdk::Monitor` via `gtk4_layer_shell::WindowExt::set_monitor(&win, Some(&monitor))`.
   - Since `Edge::Top | Edge::Bottom` anchoring already instructs the Wayland compositor to stretch the surface to the monitor's full vertical height, use `set_size_request(2, 0)` on `hot_edge_window` and `set_size_request(400, 0)` on `panel_window` so monitor hotplugs or resolution changes are never constrained by a hardcoded startup height.
3. **E4 (Peek Grace Timer):** When entering `SidebarState::Peek` from `HotEdgeHover`, start a `1500 ms` grace timer that transitions back to `Hidden` if the pointer never enters `panel_window` (cancelled as soon as the pointer enters `panel_window` or the panel transitions to `Engaged`/`Pinned`).

### P2-2 (N3, N4, N5): Notifications `dbus-monitor` Supervision & Persistent D-Bus DND Inhibit
1. **N3 & N4 (`dbus-monitor` Lifecycle):**
   - Inside `pre_exec`, after `libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL)`, check if `libc::getppid() != parent_pid` (captured before `spawn()`) and `libc::_exit(1)` if the parent already died during the fork window.
   - Store the `Child` handle, call `child.wait()` in the reader loop when stdout closes, and automatically respawn `dbus-monitor` (with a 2s backoff) if it unexpectedly exits while `nevermore-ui` is running.
2. **N5 (Fix DND Toggle):**
   - Calling `Inhibit` via a short-lived `busctl` CLI process drops the D-Bus inhibition as soon as `busctl` exits because FreeDesktop/KDE ties the inhibition cookie to the caller's unique D-Bus connection lifecycle!
   - Fix DND so it holds an active D-Bus connection (e.g., via `gio::bus_get_sync(gio::BusType::Session, ...)` + `connection.call(...)` asynchronously on the session bus held by `nevermore-ui`), storing the returned `u32` inhibit cookie and releasing it via `UnInhibit(cookie)` when toggled off or when `nevermore-ui` exits. Verify empirically with `busctl --user get-property org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications Inhibited` returning `b true` while DND is enabled!

### P2-3 (3.4 & 3.5): Read-Only `PluginContext`, Notepad Flush on Hide, Terminal Naming, & Non-Blocking GTK Thread
1. **Read-Only `PluginContext`:** Change `PluginContext::theme` from `Rc<RefCell<ThemePalette>>` to a read-only getter `ctx.theme() -> ThemePalette` (cloning or returning `Rc<ThemePalette>`), and scope `PluginContext` capabilities cleanly. Wrap plugin lifecycle calls in `std::panic::catch_unwind` (or validate manifests with `Result` instead of `expect`) so a bad plugin cannot crash the shell.
2. **Move Blocking Subprocess Calls Off the GTK Main Thread:**
   - In `Widgets`: run `playerctl metadata` in a background thread/async task and send the result back via `glib::MainContext::default().invoke_local(...)`; debounce `wpctl set-volume` (`50 ms`) and spawn asynchronously; refresh volume/mute state inside `on_show()`; and reset `last_cpu = (0, 0)` in `on_hide()` so the first tick after re-opening doesn't average across the hidden interval.
   - In `Notepad`: flush any pending unsaved text immediately inside `on_hide()` so closing/restarting within 500ms of typing never loses text!
   - In `Terminal`: use a monotonic counter (`next_page_id: Cell<usize>`) for default session names (`page_1`, `page_2`, `page_3`...) instead of `pages.len() + 1` so closing `page_1` and clicking `+ New` never collides with an existing `page_2` session; and call `term.grab_focus()` on the active terminal when `TerminalPlugin::on_show()` runs in `Engaged`/`Pinned` mode.

---

## 3. PRIORITY P3 FIXES (CSS DRIFT, CONTRAST, CONFIG MERGE, & DOCUMENTATION ACCURACY)

1. **T4, T5, T6 (Eliminate CSS Drift & Hardcoded Colors):**
   - Generate `.nm-success` (`color: {success_color}`) and `.nm-warning` (`color: {warning_color}`) in `ThemePalette::to_gtk_css()`.
   - Compute contrasting foreground text color (`#11111b` vs `#ffffff` based on relative luminance of `accent_color`) for `.nm-btn-primary` / `.nav-btn.active` so light accents like Catppuccin Mocha's `#89b4fa` have high-contrast dark text instead of 2:1 white-on-pastel.
   - Replace hardcoded hex colors in `.nav-btn:hover` and `.badge-state.engaged` with theme tokens (`text_primary`, `accent_color`, `bg_tertiary`).
   - Add rules in `to_gtk_css()` for all classes used in the plugins (`widget-header`, `clock-time`, `clock-date`, `media-title`, `media-artist`, `btn-media`, `btn-mute`, `notepad-header-title`, `btn-notepad-clear`, `term-viewport`, `bg-session*`, `btn-bg-*`), remove unused CSS selectors, delete the dead `crates/nevermore-ui/src/style.css` file, and remove the unused `ThemeTokens` struct from `crates/nevermore-core/src/types.rs`.
2. **T7 (Safe `config.toml` Preservation & Bundled Theme Sync):**
   - In `set_active_theme_name`, read existing `config.toml` into a `toml::Table` (if present), update only the `"theme"` key via `toml::to_string_pretty`, and write via a unique PID/timestamp `.tmp` file + rename so future config keys are never wiped.
   - Always refresh the 3 bundled starter themes (`nevermore-dark.toml`, `catppuccin-mocha.toml`, `tokyo-night.toml`) on startup so schema updates propagate cleanly.
   - Reload `custom.css` whenever `nevermore theme set` is executed.
3. **T8 (`docs/THEMING_GUIDE.md` & `docs/PLUGIN_SDK.md` Accuracy):**
   - Update `docs/THEMING_GUIDE.md` to accurately explain that `nevermore theme set <name>` reloads both `<name>.toml` and `custom.css`, that all fields are optional and fall back per-field to `nevermore-dark` with a logged warning if invalid, and that colors must be strict 6-digit `#rrggbb` solid hex values.

---

## 5. MANDATORY GATE TO PASS PHASE 7.5.1

1. `cargo fmt --all -- --check` (0 diffs).
2. `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings).
3. `cargo test --workspace --locked` — **must include both `nevermore-core` and new `nevermore-ui` unit tests** covering:
   - `PluginRegistry::sync_lifecycle` (visible/hidden × tab changes × same-tab re-click).
   - Notification `dbus-monitor` single-notification immediate emission + `desktop-entry` hint extraction.
   - `ThemePalette` per-field fallback, 16-color VTE guarantee, and rejection of `rgba()`/`transparent`/`../`.
4. Live empirical verification in `docs/EVIDENCE_LOG.md` (with real commit SHA recorded):
   - Hot-edge height/width and `y = 50 / 540 / 1030` trigger check.
   - Single `notify-send` immediate appearance in the Notifications tab + `org.freedesktop.Notifications.Inhibited == true` when DND is active.
   - 340px `ScrolledWindow` clamp verification.
5. Push the clean Phase 7.5.1 commit to `main`, regenerate `docs/04_FULL_SOURCE_CODE_DUMP_Phase-7_5_1.md`, upload the report (`NEVERMORE_COMM_LOG_v04_WORKER.md`) and audit snapshot to Google Drive (`Nevermore QC & AI Consultations / 2026-11-04_Phase-7_5_1_QC_Remediation`), and report the **exact commit SHA**!
