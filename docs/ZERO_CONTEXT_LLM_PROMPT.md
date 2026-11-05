# NEVERMORE PLUGIN SDK — ZERO-CONTEXT LLM SPECIFICATION & CHEAT SHEET

> Paste this entire document into any AI coding session to author a fully compliant, self-contained Nevermore plugin in a single shot.

---

## 1. Golden Rules (Violations Reject the Build)

1. **Single Prelude Import Only:**
   ```rust
   use nevermore_ui::prelude::*;
   ```
   Do NOT import raw GTK types, `Rc<RefCell<...>>`, `Command`, `tokio`, `chrono`, or `reqwest`. Everything needed is re-exported by `prelude`.
2. **Text-Free Chrome Law:**
   - Chrome UI elements MUST NOT contain static English text (no "Status:", "Volume:", "Search:", "Submit", "Settings").
   - Labels are permitted ONLY for dynamic, variable user/system data (e.g. `128 MB`, `18 ms`, `PLAYING`, `3 UNREAD`).
   - Every label displaying data MUST be created via `nm_data_label(...)` (which automatically tags `.nm-data`).
3. **No Emojis Anywhere:** Never use Unicode emojis in labels, tooltips, or logs. Use Lucide icons via `nm_icon(...)`.
4. **No Direct System Commands or Threads:**
   - NEVER call `std::process::Command::new` directly; use `Cmd::new(...)` or `Cmd::curl(...)`.
   - NEVER call `glib::timeout_add` directly; use `ctx.every(...)`.
   - NEVER use `thread::sleep` on the UI thread.
5. **Auto-Discovery by Cargo:**
   - Place your plugin in `crates/nevermore-ui/src/plugins/<id>/mod.rs`.
   - Never edit `plugins/mod.rs` or registration lists manually. `build.rs` auto-discovers your plugin if it exports `SPEC` and `build`.

---

## 2. Minimal Plugin Template (~40 Lines)

```rust
use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "hello",
    "Hello Plugin",
    "terminal",
    50,
    "Sample description of plugin capabilities",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Dynamic data label
    let data_label = nm_data_label("IDLE");
    data_label.set_tone(Tone::Muted);

    // 2. Action button with callback
    let btn = nm_icon_button("check", "Execute action");
    let label_clone = data_label.clone();
    btn.on_click(move || {
        label_clone.set("ACTIVE");
        label_clone.set_tone(Tone::Accent);
    });

    // 3. Card container
    let card = nm_card([
        data_label.widget().clone().upcast::<gtk4::Widget>(),
        btn.widget().clone().upcast::<gtk4::Widget>(),
    ]);

    // 4. Background timer (automatically paused when sidebar tab is hidden)
    let label_timer = data_label;
    ctx.every(Duration::from_secs(10), move || {
        label_timer.set("TICK");
    });

    // 5. Clamped 340px vertical page
    let page = nm_page([card.upcast::<gtk4::Widget>()]);
    page.upcast::<gtk4::Widget>()
}
```

---

## 3. The 12 UiKit Builders & Components

| Builder | Return Type | Description |
|---|---|---|
| `nm_page(children)` | `gtk4::Box` | 340px-safe vertical root container with standard padding (12px) and margins. |
| `nm_card(children)` | `gtk4::Box` | Secondary-surface rounded card container (`.nm-card`) with 8px internal spacing. |
| `nm_toolbar(left, right)` | `gtk4::Box` | Horizontal toolbar with left items, expanding spacer, and right action items. |
| `nm_icon(icon, tone, size_px)` | `TintedIcon` | Auto-recoloring SVG icon tracking theme palette changes. `.widget()` returns `Image`. |
| `nm_icon_button(icon, tooltip)` | `IconButton` | 32x32 interactive button. Use `.on_click(move \|\| ...)` to attach click handler. |
| `nm_toggle_icon_button(on, off, tip)` | `ToggleIconButton` | Toggling button with active accent style. Use `.on_toggle(move \|active\| ...)` and `.is_active()`. |
| `nm_data_label(initial_text)` | `DataLabel` | Label tagged `.nm-data`. Methods: `.set(&str)`, `.set_tone(Tone)`, `.widget()`. |
| `nm_metric_row(icon, tone)` | `MetricRow` | Composite icon + progress bar + value data label. Method: `.set(fraction_0_to_1, "value")`. |
| `nm_list()` | `gtk4::ListBox` | Compact styled list container. Append rows via `.append(&row)`. |
| `nm_list_row(icon, pri, sec, actions)` | `gtk4::ListBoxRow` | Data list item with icon, primary text, optional secondary text, and action buttons. |
| `nm_empty_state(icon, tooltip)` | `gtk4::Box` | Centered 32px muted icon for empty states without non-data text. |
| `nm_prompt_entry(icon, tip, on_submit)` | `gtk4::Box` | Enter/submit-only prompt entry that clears on submission. |
| `nm_search_entry(tooltip, debounced_fn)` | `gtk4::Box` | Search entry with search icon, clear button, and 200ms debounce. |
| `nm_status_dot(tone)` | `StatusDot` | 8x8 circular status indicator. Method: `.set_tone(Tone)`. |
| `nm_segmented(pairs, on_select)` | `gtk4::Box` | Multi-option icon-only segmented control. |
| `nm_segmented_control(pairs, init, on_select)` | `SegmentedControl` | Segmented control with programmatic `.set_active(idx)` selection. |

---

## 4. Tone System

Colors MUST NOT be hardcoded. Use the semantic `Tone` enum:
- `Tone::Primary`: Main high-contrast foreground color (`palette.text_primary`).
- `Tone::Secondary`: Secondary informational text/icon (`palette.text_secondary`).
- `Tone::Muted`: Subtle dim/disabled state (`palette.text_secondary` at 60%).
- `Tone::Accent`: Interactive highlights and active state (`palette.accent_color`).
- `Tone::Danger`: Errors, failures, destructive states (`palette.danger_color`).
- `Tone::Success`: Positive confirmations, online states (`palette.success_color`).
- `Tone::Warning`: Warnings, alerts (`palette.warning_color`).

---

## 5. PluginContext API Reference

Each plugin receives `ctx: &PluginContext` granting safe host capabilities:

### Typed Mailbox & Background Tasks
- `ctx.mailbox<Msg>(handler) -> Tx<Msg>`: Creates a channel connecting background messages to the UI thread dispatcher. `Tx<Msg>` is `Clone + Send`.
- `ctx.every(interval, callback)`: Periodic timer paused automatically when tab is hidden.
- `ctx.after_delay(duration, callback)`: Single-shot timer (e.g. for debounced sliders/inputs).
- `ctx.run(cmd, tx, map_fn)`: Runs `Cmd` in background thread and dispatches result to mailbox.
- `ctx.stream(cmd, tx, on_line, on_exit)`: Runs command streaming stdout/stderr line-by-line.
- `ctx.daemon(cmd, keep_on_hide)`: Spawns and manages a long-running background child process.

### Companion Side-Blade (500x1080 Overlay Window)
- `ctx.open_companion_blade(title, widget)`: Opens the dedicated 500x1080 companion side-blade at `margin_right = 400` on `Layer::Overlay` without auto-hiding the 400px main sidebar.
- `ctx.close_companion_blade()`: Closes the companion side-blade.

### Safe Command Builder (`Cmd`)
```rust
let cmd = Cmd::new("playerctl")
    .arg("status")
    .timeout(Duration::from_millis(500));
let res = cmd.execute_blocking(); // Returns CmdResult { code, stdout, stderr, timed_out }
```

### JSON Data Storage (Atomic & Safe)
- `ctx.load_json::<T>("state") -> Option<T>`: Loads JSON from `XDG_DATA_HOME/nevermore/plugins/<id>/state.json`. Corrupted files are safely quarantined to `.bad.<timestamp>`.
- `ctx.save_json("state", &data) -> io::Result<()>`: Atomic write via tempfile + fs::rename.

### Secret Storage
- `ctx.secret_get("token") -> Option<String>`: Reads token via system Secret Service (`secret-tool`).
- `ctx.secret_set("token", "secret123") -> io::Result<()>`: Stores token securely.

### Scoped CSS Injection
- `ctx.css("button { border-radius: 8px; }")`: Injects CSS auto-prefixed with `.nm-plugin-<id>`. Rejects literal colors (`#fff`, `rgb(...)`) — use `var(--nm-*)` or `@theme_*`.
