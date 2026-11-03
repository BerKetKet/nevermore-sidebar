# Nevermore Sidebar Plugin SDK & LLM Authoring Guide

> **Target Audience:** Developers and AI Coding Assistants (LLMs).  
> **Purpose:** Build self-contained, modular tab plugins for the Nevermore Wayland sidebar without touching core shell windowing or layer-shell logic.

---

## 1. Architecture Overview

Nevermore Sidebar is built on GTK4 and `gtk4-layer-shell` on Wayland. Every feature tab inside Nevermore is implemented as an isolated **Sidebar Plugin**.

The Nevermore shell enforces strict guardrails on all plugins:
1. **Width Clamping:** The shell wraps every plugin widget in a 340px-clamped container (`CONTENT_WIDTH = 340`, `PANEL_WIDTH = 400`). A plugin can **never** stretch the sidebar wider than 400px.
2. **Lifecycle Control:** The shell dispatches `on_show()` and `on_hide()` when the user opens, closes, or switches tabs. Plugins MUST start periodic timers (e.g., polling `/proc`) in `on_show()` and stop them in `on_hide()` to eliminate idle CPU drain.
3. **Theme Propagation:** The shell notifies plugins of live palette changes via `on_theme_changed(&palette)`. Plugins styling custom GTK widgets with CSS classes (`.nm-card`, `.nm-btn`, etc.) update automatically; custom drawing or terminal widgets can adapt dynamically.

---

## 2. Plugin Directory Layout

Every plugin lives in its own directory under `crates/nevermore-ui/src/plugins/<plugin_id>/`:

```text
crates/nevermore-ui/src/plugins/<plugin_id>/
├── plugin.toml     # Static metadata manifest
└── mod.rs          # Rust module implementing SidebarPlugin
```

### `plugin.toml` Manifest Format

```toml
id = "my-plugin"
title = "My Plugin Title"
icon = "widgets.svg"    # SVG icon name located in resources/icons/
order = 10              # Navigation rail sort order (1..9 reserved for core)
description = "Brief one-line summary of what this plugin does"
```

---

## 3. Core Types & Trait Definitions

### `PluginManifest`
```rust
pub struct PluginManifest {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub order: u32,
    pub description: String,
}
```

### `PluginContext`
Passed by the shell to `build_widget(&mut self, ctx: &PluginContext)`. Provides capability handles to system resources:
```rust
pub struct PluginContext {
    pub plugin_id: String,
    pub data_dir: PathBuf,      // $XDG_DATA_HOME/nevermore/plugins/<id>/
    pub config_dir: PathBuf,    // $XDG_CONFIG_HOME/nevermore/plugins/<id>/
    pub theme: Rc<RefCell<ThemePalette>>,
    pub tmux: TmuxBackend,
    pub systemd: SystemdTaskBackend,
    pub kwin: KWinAdapter,
}
```

### `SidebarPlugin` Trait
```rust
pub trait SidebarPlugin {
    /// Returns the static manifest for this plugin
    fn manifest(&self) -> &PluginManifest;

    /// Builds and returns the root GTK4 widget (clamped by shell to 340px width)
    fn build_widget(&mut self, ctx: &PluginContext) -> gtk4::Widget;

    /// Invoked when the sidebar opens or tab becomes visible (start timers here)
    fn on_show(&mut self) {}

    /// Invoked when sidebar hides or user switches to another tab (stop timers here)
    fn on_hide(&mut self) {}

    /// Invoked when the user switches themes at runtime via CLI or IPC
    fn on_theme_changed(&mut self, _theme: &ThemePalette) {}
}
```

---

## 4. UI Design Rules & Standard CSS Classes

Nevermore enforces a strictly solid, modern dark aesthetic. Do not use blur or transparency on widget bodies.

Use these standard semantic CSS classes for consistent theme styling:

| Class Name | Description |
|---|---|
| `.nm-card` | Container card with background secondary (`#202024`), border, and border-radius |
| `.nm-btn` | Standard secondary action button |
| `.nm-btn-primary` | Accent colored action button |
| `.nm-btn-danger` | Danger / destructive action button |
| `.nm-heading` | Bold section header text |
| `.nm-muted` | Secondary / muted text label |

---

## 5. Minimal Working Example: `HelloPlugin` (~60 lines)

### `crates/nevermore-ui/src/plugins/hello/plugin.toml`
```toml
id = "hello"
title = "Hello World"
icon = "widgets.svg"
order = 10
description = "A minimal working example Nevermore sidebar plugin"
```

### `crates/nevermore-ui/src/plugins/hello/mod.rs`
```rust
use crate::plugin_api::{PluginContext, SidebarPlugin};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Widget};
use nevermore_core::plugin::PluginManifest;
use std::cell::RefCell;
use std::rc::Rc;

pub struct HelloPlugin {
    manifest: PluginManifest,
    counter: Rc<RefCell<u32>>,
    label: Option<Label>,
    timer: RefCell<Option<glib::SourceId>>,
}

impl HelloPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest::from_toml(include_str!("plugin.toml"))
            .expect("valid plugin.toml");
        Self {
            manifest,
            counter: Rc::new(RefCell::new(0)),
            label: None,
            timer: RefCell::new(None),
        }
    }
}

impl SidebarPlugin for HelloPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn build_widget(&mut self, _ctx: &PluginContext) -> Widget {
        let card = GtkBox::new(Orientation::Vertical, 8);
        card.add_css_class("nm-card");

        let heading = Label::new(Some("HELLO NEVERMORE"));
        heading.add_css_class("nm-heading");
        card.append(&heading);

        let count_lbl = Label::new(Some("Clicks: 0"));
        count_lbl.add_css_class("nm-muted");
        self.label = Some(count_lbl.clone());
        card.append(&count_lbl);

        let btn = Button::with_label("Click Me");
        btn.add_css_class("nm-btn-primary");
        let counter_ref = self.counter.clone();
        btn.connect_clicked(move |_| {
            let mut c = counter_ref.borrow_mut();
            *c += 1;
            count_lbl.set_label(&format!("Clicks: {}", *c));
        });
        card.append(&btn);

        card.upcast::<Widget>()
    }

    fn on_show(&mut self) {
        // Start any periodic background tick strictly while visible
        if self.timer.borrow().is_none() {
            let timer_id = glib::timeout_add_seconds_local(5, move || {
                glib::ControlFlow::Continue
            });
            *self.timer.borrow_mut() = Some(timer_id);
        }
    }

    fn on_hide(&mut self) {
        // Stop timer immediately on hide to avoid CPU drain
        if let Some(id) = self.timer.borrow_mut().take() {
            id.remove();
        }
    }
}
```

---

## 6. Registering Your Plugin

Add a single line in `crates/nevermore-ui/src/plugins/mod.rs` inside `create_default_registry()`:

```rust
let ctx_hello = PluginContext::new("hello", theme.clone(), tmux.clone(), systemd.clone(), kwin.clone());
registry.register(Box::new(hello::HelloPlugin::new()), &ctx_hello);
```
