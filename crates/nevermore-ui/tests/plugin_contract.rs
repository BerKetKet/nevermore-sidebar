use gtk4::prelude::*;
use nevermore_ui::icons;
use nevermore_ui::plugin_api::PluginContext;
use nevermore_ui::sdk::lifecycle::SpecPluginAdapter;
use nevermore_ui::sdk::types::PluginSpec;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
        ("glib::timeout_add(", "Use ctx.every instead"),
        ("glib::timeout_add_local(", "Use ctx.every instead"),
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

        // 2. Check for emojis
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
    // Collect discovered plugins by testing specs
    let specs = [
        nevermore_ui::plugins::plugin_media::SPEC,
        nevermore_ui::plugins::plugin_gemini::SPEC,
        nevermore_ui::plugins::plugin_youtube::SPEC,
        nevermore_ui::plugins::plugin_vpn::SPEC,
        nevermore_ui::plugins::plugin_email::SPEC,
    ];

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
fn test_plugin_widget_tree_and_text_free_chrome() {
    if gtk4::init().is_err() {
        eprintln!("Skipping widget tree test: headless environment without display");
        return;
    }

    use gtk4::prelude::*;

    let temp = std::env::temp_dir().join(format!("nevermore_test_widget_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let ctx = PluginContext::for_test(&temp);

    type PluginBuildEntry = (&'static str, fn(&PluginContext) -> gtk4::Widget);
    let plugins: Vec<PluginBuildEntry> = vec![
        ("media", nevermore_ui::plugins::plugin_media::build),
        ("gemini", nevermore_ui::plugins::plugin_gemini::build),
        ("youtube", nevermore_ui::plugins::plugin_youtube::build),
        ("vpn", nevermore_ui::plugins::plugin_vpn::build),
        ("email", nevermore_ui::plugins::plugin_email::build),
    ];

    for (id, build_fn) in plugins {
        let widget = build_fn(&ctx);

        // Check container width clamp
        let (width, _) = widget.size_request();
        if width > 0 {
            assert!(
                width <= 340,
                "Plugin '{}' requested width {} which exceeds 340px",
                id,
                width
            );
        }

        // Recursive walk checking that all text labels have .nm-data class
        fn check_widget(w: &gtk4::Widget, plugin_id: &str) {
            if let Some(label) = w.downcast_ref::<gtk4::Label>() {
                let text = label.text();
                if !text.trim().is_empty() {
                    assert!(
                        label.has_css_class("nm-data") || label.has_css_class("nm-data-value"),
                        "Plugin '{}' contains label with text '{}' that lacks '.nm-data' CSS class!",
                        plugin_id,
                        text
                    );
                }
            }
            if let Some(button) = w.downcast_ref::<gtk4::Button>() {
                if let Some(lbl) = button.label() {
                    if !lbl.trim().is_empty() {
                        assert!(
                            button.has_css_class("nm-data"),
                            "Plugin '{}' contains button with label text '{}' that lacks '.nm-data' CSS class!",
                            plugin_id,
                            lbl
                        );
                    }
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

#[test]
fn test_plugin_lifecycle_timer_pause() {
    if gtk4::init().is_err() {
        eprintln!("Skipping lifecycle timer test: headless environment without display");
        return;
    }

    use nevermore_ui::plugin_api::SidebarPlugin;

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
