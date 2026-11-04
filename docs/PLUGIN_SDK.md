# Nevermore Sidebar Plugin SDK & LLM Authoring Guide

> **Target Audience:** Developers and AI Coding Assistants (LLMs).  
> **Purpose:** Build self-contained, modular tab plugins for the Nevermore Wayland sidebar with zero boilerplate, zero manual registration, and complete type safety.

---

## 1. Architecture Overview

Nevermore Sidebar is built on GTK4 and `gtk4-layer-shell` on Wayland. Every feature tab inside Nevermore is implemented as an isolated **Sidebar Plugin**.

The Nevermore shell enforces strict guardrails on all plugins:
1. **Width Clamping (`ScrolledWindow`):** The shell wraps every plugin widget inside a true 340px-clamped `gtk4::ScrolledWindow` (`hscrollbar_policy = Never`, `propagate_natural_width = false`, `min_content_width = 340`, `max_content_width = 340`, `hexpand = false`, `size_request = (340, -1)`). Child widgets cannot request a larger natural width and can **never** stretch the sidebar wider than 400px.
2. **Lifecycle Control:** The shell dispatches `on_show()` and `on_hide()` when the user opens, closes, or switches tabs. Timers registered with `ctx.every(...)` are automatically paused when the tab is hidden and resumed with an immediate refresh on show, ensuring zero background CPU drain. Non-persistent child processes spawned by tabs are automatically cleaned up on hide.
3. **Theme Propagation:** The shell notifies plugins of live palette changes. Plugins using the `Tone` system and `UiKit` components automatically re-color without restarting or manual paint hooks.

---

## 2. Directory Layout & Zero-Edit Auto-Discovery

Every plugin lives in its own directory under `crates/nevermore-ui/src/plugins/<plugin_id>/`:

```text
crates/nevermore-ui/src/plugins/<plugin_id>/
└── mod.rs          # Rust module exporting SPEC and build(ctx)
```

### Auto-Discovery by Cargo (`build.rs`)
You do **not** need to touch `plugins/mod.rs` or edit registration arrays!  
At build time, `crates/nevermore-ui/build.rs` automatically scans all subdirectories in `src/plugins/` (ignoring directories with a leading `_` such as `_template`). Any directory exporting `pub const SPEC: PluginSpec` is automatically compiled and registered in the sidebar's navigation rail in ascending `SPEC.order`.

---

## 3. The Single Prelude Import

Plugins import a single module:
```rust
use nevermore_ui::prelude::*;
```
This single import re-exports:
- All 12 UiKit builder functions: `nm_page`, `nm_card`, `nm_toolbar`, `nm_icon`, `nm_icon_button`, `nm_toggle_icon_button`, `nm_data_label`, `nm_metric_row`, `nm_list`, `nm_list_row`, `nm_empty_state`, `nm_search_entry`, plus `nm_status_dot`, `nm_segmented`.
- UiKit structs: `TintedIcon`, `IconButton`, `ToggleIconButton`, `DataLabel`, `MetricRow`, `StatusDot`.
- SDK core types: `PluginContext`, `PluginSpec`, `Tone`, `Cmd`, `CmdResult`, `Tx`, `KeepOnHide`.
- Essential GTK/GLib helpers: `gtk4::prelude::*`, `glib`, `gio`, `gdk`, `pango`, `Duration`, `serde::{Deserialize, Serialize}`.

---

## 4. Authoritative Plugin Example

```rust
use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "quick-note",
    "Quick Note",
    "notepad",
    12,
    "Quick scratchpad notes plugin",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Data-only label
    let status = nm_data_label("IDLE");
    status.set_tone(Tone::Muted);

    // 2. Action button with callback
    let save_btn = nm_icon_button("check", "Save note");
    let status_clone = status.clone();
    save_btn.on_click(move || {
        status_clone.set("SAVED");
        status_clone.set_tone(Tone::Success);
    });

    // 3. Card grouping
    let card = nm_card([
        status.widget().clone().upcast::<gtk4::Widget>(),
        save_btn.widget().clone().upcast::<gtk4::Widget>(),
    ]);

    // 4. Background polling timer (paused on hide)
    let status_timer = status;
    ctx.every(Duration::from_secs(10), move || {
        status_timer.set("SYNCED");
    });

    // 5. Clamped 340px vertical page
    let page = nm_page([card.upcast::<gtk4::Widget>()]);
    page.upcast::<gtk4::Widget>()
}
```

---

## 5. PluginContext API Reference

Each plugin receives `ctx: &PluginContext` granting capability-checked host access:

### Background Tasks & Mailbox
- `ctx.mailbox<Msg>(handler) -> Tx<Msg>`: Creates a typed message channel connected to the GTK main thread loop. `Tx<Msg>` is thread-safe and non-blocking (`tx.send(msg)`).
- `ctx.every(interval, callback)`: Registers a periodic timer that automatically pauses on hide and resumes immediately on show.
- `ctx.run(cmd, tx, map_fn)`: Spawns `cmd` in a worker thread and sends the mapped result to `tx`.
- `ctx.stream(cmd, tx, on_line, on_exit)`: Streams standard output and standard error line-by-line.
- `ctx.daemon(cmd, keep_on_hide)`: Spawns and manages a persistent child process.

### Safe Subprocess Execution (`Cmd`)
Never call `std::process::Command::new` directly. The `Cmd` builder handles process group creation (`process_group(0)`), pipe buffering, and enforces strict watchdog timeouts:
```rust
let cmd = Cmd::new("playerctl")
    .arg("status")
    .timeout(Duration::from_millis(500));
let res = cmd.execute_blocking(); // Returns CmdResult { code, stdout, stderr, timed_out }
```

### JSON Data Storage (Atomic & Corrupt-Safe)
- `ctx.load_json::<T>("state") -> Option<T>`: Loads deserialized state from `XDG_DATA_HOME/nevermore/plugins/<id>/state.json`. If corrupted, automatically renames the file to `.bad.<timestamp>` and returns `None` without crashing.
- `ctx.save_json("state", &data) -> io::Result<()>`: Atomically writes to a temporary file before renaming.

### Secret Management
- `ctx.secret_get("token") -> Option<String>`: Retrieves a token from the FreeDesktop Secret Service via `secret-tool`.
- `ctx.secret_set("token", "secret_val") -> io::Result<()>`: Stores a secret securely with private permissions.

### Scoped CSS Styling
- `ctx.css("button { border-radius: 8px; }") -> Result<(), String>`: Injects CSS scoped strictly to `.nm-plugin-<id>`. Literal hex or RGB colors are rejected at compile/run time — use `var(--nm-*)` or `@theme_*`.

---

## 6. The Semantic Tone System

Colors are never hardcoded. Use the semantic `Tone` enum:
- `Tone::Primary`: Main high-contrast foreground color (`palette.text_primary`).
- `Tone::Secondary`: Secondary informational text/icon (`palette.text_secondary`).
- `Tone::Muted`: Subtle dim/disabled state (`palette.text_secondary` at 60%).
- `Tone::Accent`: Interactive highlights and active state (`palette.accent_color`).
- `Tone::Danger`: Errors, failures, destructive states (`palette.danger_color`).
- `Tone::Success`: Positive confirmations, online states (`palette.success_color`).
- `Tone::Warning`: Warnings, alerts (`palette.warning_color`).

---

## 7. Contract Testing & Headless Verification

Every plugin is audited by the automated contract harness:
```bash
cargo test -p nevermore-ui --test plugin_contract
```
This contract harness executes completely headlessly without requiring a live Wayland compositor or X11 server:
1. **Static Linter:** Scans for forbidden primitives (`std::process::Command`, `timeout_add`, `tokio::`, `chrono::`, `sleep`, emojis).
2. **Manifest Validator:** Ensures unique IDs matching `^[a-z0-9-]+$` and verified Lucide icons.
3. **Text-Free Chrome Walk:** Verifies that no static English labels exist without the `.nm-data` CSS class.
4. **Lifecycle Pause Verification:** Confirms timers cleanly pause on hide and resume on show.
5. **Documentation Sync:** Validates that all public SDK symbols are fully documented.
