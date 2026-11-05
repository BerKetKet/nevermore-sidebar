use gtk4::prelude::*;
use nevermore_ui::icons;
use nevermore_ui::plugin_api::{wrap_in_plugin_container, PluginContext, SidebarPlugin};
use nevermore_ui::sdk::lifecycle::SpecPluginAdapter;
use nevermore_ui::sdk::types::PluginSpec;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::Duration;

static INIT_TEST: Once = Once::new();

fn ensure_test_display() {
    INIT_TEST.call_once(|| {
        assert!(
            std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok(),
            "WAYLAND_DISPLAY is required for plugin contract tests (silent skips are forbidden)"
        );
        gtk4::init().expect("Failed to initialize GTK4 on display");

        glib::log_set_writer_func(|level, fields| {
            if matches!(level, glib::LogLevel::Critical | glib::LogLevel::Warning) {
                let mut msg = String::new();
                for f in fields {
                    if f.key() == "MESSAGE" {
                        if let Some(val) = f.value_str() {
                            msg = val.to_string();
                        }
                    }
                }
                // Ignore transient harmless Pango font lookup warnings in headless/CI
                if !msg.is_empty() && !msg.contains("font") {
                    panic!("GLib/GTK logged {:?}: {}", level, msg);
                }
            }
            glib::LogWriterOutput::Handled
        });
    });
}

fn is_emoji(c: char) -> bool {
    matches!(c,
        '\u{1F600}'..='\u{1F64F}' | // Emoticons
        '\u{1F300}'..='\u{1F5FF}' | // Misc Symbols and Pictographs
        '\u{1F680}'..='\u{1F6FF}' | // Transport and Map
        '\u{1F1E0}'..='\u{1F1FF}' | // Flags
        '\u{2600}'..='\u{26FF}'   | // Misc symbols
        '\u{2700}'..='\u{27BF}'   | // Dingbats
        '\u{FE00}'..='\u{FE0F}'   | // Variation Selectors
        '\u{1F900}'..='\u{1F9FF}' | // Supplemental Symbols and Pictographs
        '\u{1FA70}'..='\u{1FAFF}'   // Symbols and Pictographs Extended-A
    )
}

fn collect_plugin_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip core legacy plugins
                if matches!(
                    name.as_str(),
                    "terminal" | "widgets" | "notifications" | "notepad" | "placeholders"
                ) {
                    continue;
                }
                files.extend(collect_plugin_rs_files(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn test_plugin_source_linter() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let plugins_dir = manifest_dir.join("src").join("plugins");
    let files = collect_plugin_rs_files(&plugins_dir);

    assert!(
        !files.is_empty(),
        "No plugin source files discovered in {:?}",
        plugins_dir
    );

    let forbidden_patterns = [
        ("std::process::Command", "Use Cmd builder instead"),
        ("Command::new", "Use Cmd builder instead"),
        ("glib::timeout_add(", "Use ctx.every instead"),
        ("glib::timeout_add_local(", "Use ctx.every instead"),
        ("timeout_add", "Use ctx.every instead"),
        (
            "execute_blocking",
            "Never block on GTK main thread; use ctx.run or background thread",
        ),
        (
            "tokio::",
            "Do not pull in tokio runtime; use ctx.run or glib",
        ),
        (
            "async_std::",
            "Do not pull in async_std; use ctx.run or glib",
        ),
        ("smol::", "Do not pull in smol; use ctx.run or glib"),
        ("reqwest::", "Use Cmd::curl or ctx.run instead"),
        ("ureq::", "Use Cmd::curl or ctx.run instead"),
        ("chrono::", "Use std::time::SystemTime / Duration instead"),
    ];

    for file in files {
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", file, e));

        // 1. Check forbidden primitives
        for (pat, reason) in &forbidden_patterns {
            assert!(
                !content.contains(pat),
                "Forbidden pattern '{}' found in {:?}: {}",
                pat,
                file,
                reason
            );
        }

        // 2. Check for literal hex/rgba inside plugin CSS calls
        if content.contains("ctx.css(") {
            assert!(
                !content.contains("rgba(") && !content.contains("rgb("),
                "Plugin CSS in {:?} must not contain hardcoded rgba/rgb; use CSS custom properties",
                file
            );
        }

        // 3. Check for emojis
        for (line_idx, line) in content.lines().enumerate() {
            for c in line.chars() {
                assert!(
                    !is_emoji(c),
                    "Forbidden emoji character '{}' found at {:?}:{}: {}",
                    c,
                    file,
                    line_idx + 1,
                    line
                );
            }
        }
    }
}

#[test]
fn test_plugin_manifest_and_icons() {
    let specs = nevermore_ui::plugins::ALL_DISCOVERED_SPECS;

    let mut ids = HashSet::new();
    let mut orders = HashSet::new();

    for spec in specs {
        // ID check: non-empty, lowercase alphanumeric + hyphens
        assert!(!spec.id.is_empty(), "Plugin ID cannot be empty");
        assert!(
            spec.id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "Plugin ID '{}' must be lowercase alphanumeric or hyphens",
            spec.id
        );
        assert!(ids.insert(spec.id), "Duplicate plugin ID '{}'", spec.id);

        // Title check
        assert!(!spec.title.is_empty(), "Plugin title cannot be empty");

        // Icon check: must exist in icon library
        assert!(
            icons::has_icon(spec.icon),
            "Plugin '{}' references unknown icon '{}'",
            spec.id,
            spec.icon
        );

        // Order check
        assert!(
            orders.insert(spec.order),
            "Duplicate plugin order '{}' for ID '{}'",
            spec.order,
            spec.id
        );
    }
}

#[test]
fn test_plugin_gtk_contract_suite() {
    ensure_test_display();

    // 1. Widget tree and text-free chrome
    run_widget_tree_and_text_free_chrome();

    // 2. Lifecycle pump and cleanup (show -> pump 100ms -> hide -> show -> hide -> on_theme_changed)
    run_lifecycle_pump_and_cleanup();

    // 3. Lifecycle timer pause
    run_lifecycle_timer_pause();
}

fn run_widget_tree_and_text_free_chrome() {
    let temp = std::env::temp_dir().join(format!("nevermore_test_widget_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let ctx = PluginContext::for_test(&temp);

    let plugins = nevermore_ui::plugins::ALL_DISCOVERED_PLUGINS;

    for (spec, build_fn) in plugins {
        let id = spec.id;
        let widget = build_fn(&ctx);

        // Check container width clamp inside ScrolledWindow wrapper
        let container = wrap_in_plugin_container(&widget, false);
        let (min_w, _) = container.preferred_size();
        assert!(
            min_w.width() <= 340,
            "Plugin '{}' requested min width {} which exceeds 340px clamp",
            id,
            min_w.width()
        );

        // Recursive walk checking that all text labels have .nm-data class
        fn check_widget(w: &gtk4::Widget, plugin_id: &str) {
            if w.is::<gtk4::Entry>() || w.is::<gtk4::PasswordEntry>() {
                return;
            }

            if let Some(label) = w.downcast_ref::<gtk4::Label>() {
                let text = label.text();
                if !text.trim().is_empty() {
                    for c in text.chars() {
                        assert!(
                            !is_emoji(c),
                            "Plugin '{}' contains forbidden emoji character '{}' in label '{}'",
                            plugin_id,
                            c,
                            text
                        );
                    }
                    assert!(
                        label.has_css_class("nm-data")
                            || label.has_css_class("nm-data-value")
                            || label.has_css_class("nm-section-header")
                            || label.has_css_class("nm-field-label"),
                        "Plugin '{}' contains label with text '{}' that lacks required CSS class (.nm-data, .nm-section-header, or .nm-field-label)!",
                        plugin_id,
                        text
                    );
                }
            }
            if let Some(button) = w.downcast_ref::<gtk4::Button>() {
                if let Some(lbl) = button.label() {
                    assert!(
                        lbl.trim().is_empty(),
                        "Plugin '{}' contains button with non-empty text label '{}'! Static chrome buttons must be text-free icon buttons.",
                        plugin_id,
                        lbl
                    );
                }
            }

            let mut child = w.first_child();
            while let Some(c) = child {
                check_widget(&c, plugin_id);
                child = c.next_sibling();
            }
        }

        check_widget(&widget, id);
    }

    let _ = fs::remove_dir_all(&temp);
}

fn run_lifecycle_pump_and_cleanup() {
    let temp = std::env::temp_dir().join(format!("nevermore_test_pump_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let ctx = PluginContext::for_test(&temp);

    let plugins = nevermore_ui::plugins::ALL_DISCOVERED_PLUGINS;

    for (spec, build_fn) in plugins {
        let mut adapter = SpecPluginAdapter::new(*spec, *build_fn, &ctx);
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _widget = adapter.build_widget(&ctx);

            adapter.on_show();
            // pump main loop for 100ms
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                glib::MainContext::default().iteration(false);
            }

            adapter.on_hide();
            assert_eq!(
                ctx.active_timer_count(),
                0,
                "Plugin '{}' leaked active timers after first hide!",
                spec.id
            );

            adapter.on_show();
            adapter.on_hide();
            assert_eq!(
                ctx.active_timer_count(),
                0,
                "Plugin '{}' leaked active timers after second hide!",
                spec.id
            );

            let dummy_palette = nevermore_core::theme::ThemePalette::default();
            adapter.on_theme_changed(&dummy_palette);
        }));
        assert!(
            res.is_ok(),
            "Plugin '{}' panicked or failed timer cleanup during lifecycle test",
            spec.id
        );
    }

    let _ = fs::remove_dir_all(&temp);
}

fn run_lifecycle_timer_pause() {
    let temp =
        std::env::temp_dir().join(format!("nevermore_test_lifecycle_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let ctx = PluginContext::for_test(&temp);

    let spec = PluginSpec::new("lifecycle_test", "Test", "terminal", 100, "desc");

    fn build_test(ctx: &PluginContext) -> gtk4::Widget {
        ctx.every(Duration::from_millis(50), || {});
        gtk4::Box::new(gtk4::Orientation::Vertical, 0).upcast()
    }

    let mut adapter = SpecPluginAdapter::new(spec, build_test, &ctx);
    let _ = adapter.build_widget(&ctx);

    assert_eq!(ctx.active_timer_count(), 0);

    adapter.on_show();
    assert_eq!(ctx.active_timer_count(), 1);

    adapter.on_hide();
    assert_eq!(ctx.active_timer_count(), 0);

    adapter.on_show();
    assert_eq!(ctx.active_timer_count(), 1);

    adapter.on_hide();
    assert_eq!(ctx.active_timer_count(), 0);

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_plugin_documentation_sync() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc_path = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
        .join("ZERO_CONTEXT_LLM_PROMPT.md");

    assert!(
        doc_path.exists(),
        "ZERO_CONTEXT_LLM_PROMPT.md does not exist at {:?}",
        doc_path
    );
    let content = fs::read_to_string(&doc_path).expect("Failed to read documentation");

    let required_symbols = [
        "nm_page",
        "nm_card",
        "nm_toolbar",
        "nm_icon",
        "nm_icon_button",
        "nm_toggle_icon_button",
        "nm_data_label",
        "nm_metric_row",
        "nm_list",
        "nm_list_row",
        "nm_empty_state",
        "nm_search_entry",
        "nm_status_dot",
        "nm_segmented",
        "ctx.every",
        "ctx.run",
        "ctx.stream",
        "ctx.daemon",
        "ctx.mailbox",
        "ctx.load_json",
        "ctx.save_json",
        "ctx.secret_get",
        "ctx.secret_set",
        "ctx.css",
        "Tone::Primary",
        "Tone::Secondary",
        "Tone::Muted",
        "Tone::Accent",
        "Tone::Danger",
        "Tone::Success",
        "Tone::Warning",
    ];

    for sym in required_symbols {
        assert!(
            content.contains(sym),
            "Documentation missing required SDK symbol: '{}'",
            sym
        );
    }
}
