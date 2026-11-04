# [MANAGER AI DIRECTIVE — `v07_MANAGER`]: PHASE 7.5.2 SIGN-OFF → EXECUTE PHASE 7.6 (ZERO-CONTEXT PLUGIN SDK, UIKIT, CONTRACT HARNESS, & SEED THEME ENGINE)

**From:** Manager AI & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Base Commit on `main`:** `6444e4d` (code `3c08d0b`)  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v07_MANAGER.md` (synced to Google Drive `1rPDOAszW8XLARbE4dyGYCNYSm5Izy6Du`)

---

## 0. PHASE 7.5.2 SIGN-OFF & CRITICAL HOTFIX (`Layer::Overlay` -> `Layer::Top`)

Phase 7.5.2 (`3c08d0b` / `6444e4d`) is **APPROVED**, with one mandatory 2-line hotfix before starting the Phase 7.6 SDK work:
- **Hotfix 7.6.0 (`crates/nevermore-ui/src/main.rs` lines 85 & 114):**
  - In `3c08d0b`, `hot_edge_window.set_layer` and `panel_window.set_layer` were changed from `Layer::Top` to `Layer::Overlay`.
  - **Fix Immediately:** Change both back to `Layer::Top` (while keeping `set_exclusive_zone(-1)`). Per `AGENTS.md` Fixed Decision #4 and Spike 3, `Layer::Overlay` is strictly forbidden because it paints over active fullscreen games and steals right-edge clicks during gameplay, whereas `Layer::Top` + `exclusive_zone = -1` spans the full 1080p height while still allowing fullscreen games to occlude the sidebar.

---

## 1. STRATEGIC GOAL OF PHASE 7.6: ZERO-CONTEXT AI PLUGIN & THEME AUTHORING

Before building the remaining feature tabs (`vpn`, `media`, `gemini`, `youtube`, `email`), we are upgrading `nevermore-ui` and `nevermore-core` so that **a brand-new, zero-context external AI session** can be handed a single 1-page prompt (`docs/ZERO_CONTEXT_LLM_PROMPT.md`) and produce a ~50-line plugin (`crates/nevermore-ui/src/plugins/<id>/mod.rs`) or theme that compiles on the first try with:
- **Zero `Rc<RefCell<...>>`** in plugin code (`ctx.mailbox`).
- **Zero manual icon re-coloring or `on_theme_changed` hooks** (`Tone` enum + auto-tinted icons).
- **Zero manual `SourceId` timer tracking** (`ctx.every` managed by the shell on show/hide).
- **Zero raw `Command::new` / thread / channel boilerplate** (`ctx.run`, `ctx.stream`, `ctx.daemon`, `ctx.secret_get`/`set`).
- **Zero manual registration edits to `plugins/mod.rs`** (`build.rs` auto-discovery).
- **Fast, actionable self-correction via `cargo test -p nevermore-ui --test plugin_contract`**.

Execute Tasks 1 through 6 below on `main`:

---

## 2. PHASE 7.6 IMPLEMENTATION TASKS

### TASK 1: `nevermore_ui::prelude` + `PluginSpec` + `build.rs` Auto-Registration
1. **Expose Library Crate Target (`crates/nevermore-ui/src/lib.rs`):**
   - Export `pub mod sdk; pub mod prelude; pub mod plugin_api; pub mod plugins; pub mod icons; pub mod nav_rail; pub mod ipc_server;` from `crates/nevermore-ui/src/lib.rs` so integration tests (`tests/plugin_contract.rs`) and plugins can `use crate::prelude::*;` (or `use nevermore_ui::prelude::*;`).
   - `prelude` must re-export: `gtk4::{self, glib, gio, gdk, pango, prelude::*}`, `serde::{Serialize, Deserialize}`, `serde_json`, `std::time::Duration`, `PluginContext`, `PluginSpec`, `Tone`, `Cmd`, `CmdResult`, `Tx`, `KeepOnHide`, and all `UiKit` builders (`nm_page`, `nm_card`, etc.).
2. **`PluginSpec` Adapter (Alongside Existing `SidebarPlugin` Trait):**
   - Keep `SidebarPlugin` working for the 4 existing plugins (`terminal`, `widgets`, `notifications`, `notepad`).
   - Define a zero-boilerplate `PluginSpec`:
     ```rust
     pub struct PluginSpec {
         pub id: &'static str,
         pub title: &'static str,
         pub icon: &'static str,
         pub order: u32,
         pub description: &'static str,
     }
     impl PluginSpec {
         pub const fn new(id: &'static str, title: &'static str, icon: &'static str, order: u32, description: &'static str) -> Self { ... }
     }
     ```
   - A new-style plugin only needs to export `pub const SPEC: PluginSpec = ...;` and `pub fn build(ctx: &PluginContext) -> gtk4::Widget` in `crates/nevermore-ui/src/plugins/<id>/mod.rs`!
   - Wrap it in a shell-side `SpecPluginAdapter` implementing `SidebarPlugin` that automatically starts/stops all `ctx.every(...)` timers and `ctx.stream(...)` processes registered on that plugin's `PluginContext` during `on_show()` and `on_hide()`!
3. **`crates/nevermore-ui/build.rs` Auto-Discovery:**
   - Scan `crates/nevermore-ui/src/plugins/*/mod.rs`, skipping any directory starting with `_` (like `_template`) and skipping the 4 legacy trait plugins (`terminal`, `widgets`, `notifications`, `notepad`, which stay registered in `plugins/mod.rs`).
   - If a `src/plugins/<id>/mod.rs` file contains `pub const SPEC: PluginSpec`, generate `$OUT_DIR/plugins_gen.rs` declaring `#[path = "..."] pub mod <id>;` and `pub fn register_discovered_spec_plugins(registry: &mut PluginRegistry, ...)`.
   - Emit `cargo:rerun-if-changed=src/plugins` and `cargo:rerun-if-changed=resources/icons`.
   - Also scan `crates/nevermore-ui/resources/icons/*.svg` in `build.rs` so dropping any `<name>.svg` file into `resources/icons/` automatically adds it to the embedded icon lookup table!
   - Convert the 5 remaining placeholder tabs (`media`, `gemini`, `youtube`, `vpn`, `email`) or create `crates/nevermore-ui/src/plugins/_template/mod.rs` using the new `SPEC` + `build(ctx)` API to prove `build.rs` auto-discovery works end-to-end.

---

### TASK 2: Auto-Themed Icon Widget (`Tone` Enum + Zero Plugin Re-Coloring)
1. **`Tone` Enum (`crates/nevermore-ui/src/sdk/types.rs`):**
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum Tone {
       Primary,
       Secondary,
       Muted,
       Accent,
       Danger,
       Success,
       Warning,
   }
   ```
2. **Eliminate Manual `on_theme_changed` Icon Re-Coloring:**
   - Implement `nm_icon(icon_spec: impl Into<IconSource>, tone: Tone, size_px: i32) -> TintedIcon` (and use it inside `nm_icon_button`, `nm_toggle_icon_button`, `nm_metric_row`, `nm_empty_state`, etc.).
   - Support both named Lucide icons (`"cpu"`, `"shield"`, `"play"`, etc.) and inline custom SVGs (`IconSource::Svg(include_str!(...))`).
   - Either:
     - **Approach A:** Use a `gtk4::DrawingArea` / custom paintable that reads `widget.color()` (set via CSS classes `.nm-tone-primary`, `.nm-tone-secondary`, `.nm-tone-accent`, `.nm-tone-danger`, `.nm-tone-success`, `.nm-tone-warning` and CSS custom properties `--nm-text-primary`, `--nm-accent`, etc. in `ThemePalette::to_gtk_css()`) and masks the cached SVG alpha surface, OR
     - **Approach B:** Register each created `nm_icon` / `nm_icon_button` weak widget reference in a thread-local theme-observer list inside `icons.rs` so whenever `SetTheme` or `set_tone` or `:active`/toggled state changes, `icons.rs` automatically updates the `gtk4::Image` texture with the exact `ThemePalette` hex color (`contrast_text_color_for_hex` when on an accent background)!
   - **Result:** Plugin code never touches hex colors or implements `on_theme_changed`!

---

### TASK 3: PluginContext Zero-Boilerplate Helpers (mailbox, run, every, json, secret)
Add the following ergonomic methods to `PluginContext` (all closures wrapped in `std::panic::catch_unwind(AssertUnwindSafe(...))` so a plugin panic never aborts the GTK FFI boundary):
1. **`ctx.mailbox::<Msg>(handler)`:**
   - Spawns a local async loop (`glib::spawn_future_local`) owning the `FnMut(Msg, &Tx<Msg>)` handler by move.
   - Returns `Tx<Msg>` (`Clone + Send + Sync + 'static` wrapping `async_channel::Sender<Msg>`) with a non-blocking `.send(msg: Msg)` method.
   - Because all messages (`on_click`, `every` ticks, `ctx.run` command completions) arrive sequentially on the main thread into this single `FnMut`, plugins need zero `Rc<RefCell<...>>`!
2. **`ctx.every(interval: Duration, callback: impl Fn() + 'static)`:**
   - Registers a repeating timer in the plugin's lifecycle controller.
   - When the plugin is shown (`on_show`), the shell automatically fires `callback()` once immediately and starts `glib::timeout_add_local(interval, ...)`.
   - When the plugin is hidden (`on_hide`), the shell automatically removes the `SourceId` so zero timers ever run while hidden!
3. **`ctx.run(cmd: Cmd, tx: &Tx<Msg>, map_res: impl FnOnce(CmdResult) -> Msg + Send + 'static)`:**
   - `Cmd::new(program).arg(...).args(...).env(k, v).stdin_bytes(...).timeout(Duration::from_secs(2))` (plus `Cmd::curl(url)` helper).
   - Runs off the GTK main thread via `gio::spawn_blocking`.
   - Spawns the subprocess in its own process group (`process_group(0)`), enforces the timeout (killing the entire process group with `kill(-pgid, SIGKILL)` if timed out), waits/reaps the child (`.wait_with_output()`), and sends `map_res(CmdResult { code, stdout, stderr, timed_out })` into `tx`.
4. **`ctx.stream(cmd, &tx, on_line, on_exit)` & `ctx.daemon(cmd, policy)`:**
   - Spawns long-lived child processes from the main thread (so `prctl(PR_SET_PDEATHSIG, SIGKILL)` is bound to the main `nevermore-ui` process thread and NOT a transient `gio::spawn_blocking` pool thread!) and reads stdout / waits on a background thread.
   - By default, kills the child on `on_hide` unless `KeepOnHide` is specified.
5. **`ctx.load_json::<T: DeserializeOwned + Default>(name: &str) -> T` & `ctx.save_json<T: Serialize>(name: &str, val: &T)`:**
   - Validates `name` against `^[a-zA-Z0-9._-]{1,64}$` (blocking `..` traversal).
   - `load_json` returns `T::default()` if missing; if corrupt, logs a warning, renames the corrupt file to `<name>.bad`, and returns `T::default()`.
   - `save_json` serializes on the main thread and writes atomically (`tmp` file + `sync_all` + `fs::rename`) on a background worker with a monotonic sequence guard.
6. **`ctx.secret_get(key, &tx, map)` & `ctx.secret_set(key, secret_val, &tx, map)`:**
   - Wraps `secret-tool lookup/store --label="Nevermore" service nevermore plugin <plugin_id> key <key>`, passing `secret_val` over stdin (never in argv!).
7. **`ctx.css(scoped_css: &str)`:**
   - Validates that `scoped_css` contains no literal `#rrggbb`, `rgb()`, `rgba()`, or `transparent` (must use `var(--nm-*)` custom properties) and automatically prefixes rules with `.nm-plugin-<id>`.

---

### TASK 4: UiKit — 12 340px-Safe Widget Builders (`crates/nevermore-ui/src/sdk/ui_kit.rs`)
Implement and export in `nevermore_ui::prelude::*`:
1. `nm_page(children)` — Vertical GtkBox (spacing = 10, 12px margins, hexpand = true, `.nm-plugin-<id>` class).
2. `nm_card(children)` — Vertical GtkBox with `.nm-card` class (spacing = 8).
3. `nm_toolbar(left_children, right_children)` — Horizontal GtkBox with left items, expanding spacer, and right items.
4. `nm_icon(icon, tone, size_px) -> TintedIcon` (with `.set_tone(Tone)` and `.set_icon(name)`).
5. `nm_icon_button(icon, tooltip) -> IconButton` (sets tooltip + accessible label automatically, `.on_click(move || ...)`).
6. `nm_toggle_icon_button(on_icon, off_icon, tooltip) -> ToggleIconButton` (with `.is_active()`, `.set_active_silent(bool)` that never re-triggers `.on_click`, and automatic accent styling when active).
7. `nm_data_label(initial_text) -> DataLabel` (adds `.nm-data` marker class for the contract test, sets `EllipsizeMode::End`, `xalign = 0.0`, `.set(text)` and `.set_tone(Tone)`).
8. `nm_metric_row(icon, tone) -> MetricRow` (icon + ProgressBar + `.nm-data` value label, driven by `.set(fraction_0_to_1, text)`).
9. `nm_list()` & `nm_list_row(icon, primary_data, secondary_data, action_widgets)`.
10. `nm_empty_state(icon, tooltip)` — Centered 32px muted icon for empty lists/states.
11. `nm_search_entry(tooltip, on_change_debounced)` — Search icon + text entry (no static placeholder text, tooltip only) + clear icon button + 200ms debounce.
12. `nm_status_dot(tone) -> StatusDot` (8x8 rounded status indicator colored by Tone, `.set_tone(Tone)`) and `nm_segmented(icon_tooltip_pairs, on_select)`.

---

### TASK 5: `tests/plugin_contract.rs` Automated Self-Correction Test Suite
Create `crates/nevermore-ui/tests/plugin_contract.rs` so running `cargo test -p nevermore-ui --test plugin_contract` automatically tests every registered plugin (plus `_template`) and outputs clear, LLM-actionable error messages:
1. **Source-Code Static Linter (Runs 100% Headless even without a Display!):**
   - Scans every `crates/nevermore-ui/src/plugins/<id>/**/*.rs` file (for new SPEC plugins) and fails with an explicit fix message if it finds:
     - `Command::new` -> *"Use ctx.run(Cmd::new(...)) or ctx.stream(...) instead of raw std::process::Command::new."*
     - `timeout_add` / `timeout_add_seconds` -> *"Use ctx.every(Duration, ...) so timers automatically stop when the sidebar is hidden."*
     - `tokio::` / `reqwest::` / `chrono::` / `thread::sleep` -> *"Banned crate/call on UI thread; use nevermore_ui::prelude::*, ctx.run(Cmd::curl(...)), or glib::DateTime."*
     - Any emoji characters (`U+1F300..1FAFF`, `U+2600..27BF`).
2. **Manifest & Icon Validation (Headless):**
   - Verifies `id` regex `^[a-z0-9_-]{1,32}$`, unique `id`, non-empty title, and that `manifest.icon` exists in the embedded icon table.
3. **Widget Tree, Width Clamp, & Lifecycle Test (Runs when Wayland/X11 display is active; skips with warning if DISPLAY and WAYLAND_DISPLAY are empty):**
   - Builds each plugin with `PluginContext::for_test(temp_dir)`.
   - **Text-Free Chrome Walk:** Walks the widget tree via `first_child()` / `next_sibling()` and fails if any `gtk4::Label` has non-empty text without the `.nm-data` CSS class, or if any `gtk4::Button` has a text label (`btn.label().is_some()`) without `.nm-data`!
   - **340px Width Clamp Check:** Wraps the widget in `wrap_in_plugin_container` and asserts `container.measure(Orientation::Horizontal, -1).0 <= 340`.
   - **Lifecycle & Timer Leak Check:** Calls `on_show()`, pumps GLib main context for 50ms, calls `on_hide()`, and asserts that `active_timer_count() == 0` while hidden!
4. **Documentation Sync Test:**
   - Asserts that `docs/ZERO_CONTEXT_LLM_PROMPT.md` mentions all core UiKit and PluginContext helper functions so the prompt can never drift out of sync with the code.

---

### TASK 6: Theme Engine Upgrades (`nevermore theme check`, `expand`, & `[seed]` OKLab Derivation)
1. **CSS Custom Properties (`var(--nm-*)`):**
   - In `ThemePalette::to_gtk_css()`, emit `--nm-bg-primary`, `--nm-bg-secondary`, `--nm-bg-tertiary`, `--nm-border`, `--nm-text-primary`, `--nm-text-secondary`, `--nm-accent`, `--nm-accent-fg`, `--nm-danger`, `--nm-success`, `--nm-warning` on `window, .sidebar-panel` alongside the existing classes, plus `.nm-tone-primary` through `.nm-tone-warning`.
2. **`[seed]` Palette Derivation in OKLab (`crates/nevermore-core/src/theme.rs`):**
   - If a theme `.toml` file contains a `[seed]` table (`bg = "#..."`, `fg = "#..."`, `accent = "#..."`, plus optional `danger`, `success`, `warning`), automatically derive all missing surface (`bg_secondary`, `bg_tertiary`, `border_color`), text (`text_secondary`), and 16-color `term_palette` fields using perceptual OKLab mixing (~40 lines of pure Rust math, zero new crates!). Any explicitly provided top-level key overrides the seed-derived value.
3. **CLI Validator & Expander (`nevermore theme check <path> [--json]` & `nevermore theme expand <path>`):**
   - Add `nevermore theme check <file.toml> [--json]` to `nevermore-cli` and `nevermore-core`:
     - Strictly validates all keys (reporting unknown keys with Levenshtein edit-distance *"did you mean ...?"* suggestions), hex formats, dark-theme luminance ceiling (`bg_primary` luminance < 0.15), and WCAG contrast ratios (`text_primary` on `bg_primary` ≥ 4.5:1, `text_secondary` on `bg_secondary` ≥ 3.0:1, `accent_color` on `bg_tertiary` ≥ 3.0:1). Exits with code 1 if any check fails.
   - Add `nevermore theme expand <file.toml>` to print the full expanded TOML from a `[seed]` file.

---

## 3. DELIVERABLES & GATE VERIFICATION (`NEVERMORE_COMM_LOG_v08_WORKER.md`)

1. Write `docs/ZERO_CONTEXT_LLM_PROMPT.md` — the concise, self-contained 1-Page Prompt Template (Hard Rules + Exact `nevermore_ui::prelude` API Cheat-Sheet + the 45-line `_template` example + `[seed]` theme spec) ready for `bgg` to paste into a brand-new external AI session.
2. Update `docs/PLUGIN_SDK.md` and `docs/THEMING_GUIDE.md`.
3. Run and log in `docs/EVIDENCE_LOG.md`:
   - `cargo fmt --all -- --check` (0 diffs)
   - `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings)
   - `cargo test --workspace --locked` (including `cargo test -p nevermore-ui --test plugin_contract`)
   - `nevermore theme check` and `nevermore theme expand` outputs on a sample `[seed]` theme.
4. Commit code first on `main`, record the exact SHA, commit docs/dump (`docs/06_FULL_SOURCE_CODE_DUMP_Phase-7_6.md`), push to `origin/main`, and upload `NEVERMORE_COMM_LOG_v08_WORKER.md`, `ZERO_CONTEXT_LLM_PROMPT.md`, and the dump into a new Google Drive subfolder `Nevermore QC & AI Consultations / 2026-11-04_Phase-7_6_SDK` (under `1rPDOAszW8XLARbE4dyGYCNYSm5Izy6Du`).
