use crate::nav_rail::NavRail;
use crate::plugin_api::{wrap_in_plugin_container, PluginContext};
use crate::plugins::PluginRegistry;
use crate::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Entry, Orientation, SpinButton, StringList, Switch};
use nevermore_core::theme::{
    get_active_theme_name, get_appearance_config, get_disabled_plugins, list_available_themes,
    load_theme, set_active_theme_name, set_appearance_config, set_plugin_disabled, themes_dir,
    SeedConfig, ThemePalette,
};
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

#[allow(clippy::type_complexity)]
pub struct SettingsPageContext {
    pub theme: Rc<RefCell<ThemePalette>>,
    pub nav_rail: Rc<RefCell<Option<Rc<NavRail>>>>,
    pub registry: Rc<RefCell<PluginRegistry>>,
    pub on_theme_updated: Rc<dyn Fn(&ThemePalette)>,
    pub on_appearance_updated: Rc<dyn Fn(&str, &str, f64)>,
    pub tmux: nevermore_core::TmuxBackend,
    pub systemd: nevermore_core::SystemdTaskBackend,
    pub kwin: nevermore_core::KWinAdapter,
}

pub fn build_settings_page(s_ctx: Rc<SettingsPageContext>) -> gtk4::ScrolledWindow {
    let mut page_cards = Vec::new();

    // ==========================================
    // CARD 1: APPEARANCE & THEME
    // ==========================================
    let card1_header = nm_section_header("APPEARANCE & THEME");
    let theme_label = nm_field_label("Active Theme");

    let themes = list_available_themes();
    let current_theme_name = get_active_theme_name();
    let theme_strings: Vec<&str> = themes.iter().map(|s| s.as_str()).collect();
    let string_list = StringList::new(&theme_strings);
    let theme_dropdown = DropDown::new(Some(string_list), None::<gtk4::Expression>);

    if let Some(pos) = themes.iter().position(|t| t == &current_theme_name) {
        theme_dropdown.set_selected(pos as u32);
    }

    let s_ctx_theme = s_ctx.clone();
    let themes_list = themes;
    theme_dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected() as usize;
        if idx < themes_list.len() {
            let selected_name = &themes_list[idx];
            if let Ok(new_palette) = load_theme(selected_name) {
                let _ = set_active_theme_name(selected_name);
                (s_ctx_theme.on_theme_updated)(&new_palette);
            }
        }
    });

    // OKLab Seed Hex Inputs
    let seed_label = nm_field_label("Derive from OKLab Seed (Hex)");
    let seed_box = GtkBox::new(Orientation::Horizontal, 6);
    seed_box.set_hexpand(true);

    let cur_theme = s_ctx.theme.borrow().clone();
    let bg_entry = Entry::new();
    bg_entry.set_placeholder_text(Some("#18181b"));
    bg_entry.set_text(&cur_theme.bg_primary);
    bg_entry.set_width_chars(8);
    bg_entry.set_hexpand(true);

    let fg_entry = Entry::new();
    fg_entry.set_placeholder_text(Some("#f4f4f5"));
    fg_entry.set_text(&cur_theme.text_primary);
    fg_entry.set_width_chars(8);
    fg_entry.set_hexpand(true);

    let accent_entry = Entry::new();
    accent_entry.set_placeholder_text(Some("#89b4fa"));
    accent_entry.set_text(&cur_theme.accent_color);
    accent_entry.set_width_chars(8);
    accent_entry.set_hexpand(true);

    let seed_apply_btn = nm_icon_button("check", "Apply OKLab Seed");

    seed_box.append(&bg_entry);
    seed_box.append(&fg_entry);
    seed_box.append(&accent_entry);
    seed_box.append(seed_apply_btn.widget());

    let s_ctx_seed = s_ctx.clone();
    let bg_weak = bg_entry.downgrade();
    let fg_weak = fg_entry.downgrade();
    let acc_weak = accent_entry.downgrade();

    seed_apply_btn.on_click(move || {
        let (bg, fg, acc) = match (bg_weak.upgrade(), fg_weak.upgrade(), acc_weak.upgrade()) {
            (Some(b), Some(f), Some(a)) => (
                b.text().to_string(),
                f.text().to_string(),
                a.text().to_string(),
            ),
            _ => return,
        };

        if !is_valid_hex(&bg) || !is_valid_hex(&fg) || !is_valid_hex(&acc) {
            eprintln!("[nevermore-settings] invalid hex format in seed inputs");
            return;
        }

        let seed = SeedConfig {
            bg: bg.clone(),
            fg: fg.clone(),
            accent: acc.clone(),
            danger: None,
            success: None,
            warning: None,
        };

        let mut derived = ThemePalette::from_seed(&seed);
        derived.name = "custom-seed".to_string();

        let t_dir = themes_dir();
        if let Ok(()) = fs::create_dir_all(&t_dir) {
            if let Ok(toml_content) = toml::to_string_pretty(&derived) {
                let _ = fs::write(t_dir.join("custom-seed.toml"), toml_content);
            }
        }

        let _ = set_active_theme_name("custom-seed");
        (s_ctx_seed.on_theme_updated)(&derived);
    });

    let card1 = nm_card([
        card1_header.upcast::<gtk4::Widget>(),
        theme_label.upcast::<gtk4::Widget>(),
        theme_dropdown.upcast::<gtk4::Widget>(),
        seed_label.upcast::<gtk4::Widget>(),
        seed_box.upcast::<gtk4::Widget>(),
    ]);
    page_cards.push(card1.upcast::<gtk4::Widget>());

    // ==========================================
    // CARD 2: TYPOGRAPHY & SYSTEM FONTS
    // ==========================================
    let card2_header = nm_section_header("TYPOGRAPHY & SYSTEM FONTS");

    // Enumerate system font families via Pango
    let dummy_widget = GtkBox::new(Orientation::Horizontal, 0);
    let families = dummy_widget.pango_context().list_families();
    let mut font_names: Vec<String> = families.iter().map(|f| f.name().to_string()).collect();
    font_names.sort();
    font_names.dedup();

    let font_str_slice: Vec<&str> = font_names.iter().map(|s| s.as_str()).collect();

    let ui_font_label = nm_field_label("UI Font Family");
    let ui_string_list = StringList::new(&font_str_slice);
    let ui_font_dropdown = DropDown::new(Some(ui_string_list), None::<gtk4::Expression>);

    let mono_font_label = nm_field_label("Monospace Font Family");
    let mono_string_list = StringList::new(&font_str_slice);
    let mono_font_dropdown = DropDown::new(Some(mono_string_list), None::<gtk4::Expression>);

    let (cfg_ui, cfg_mono, cfg_size) = get_appearance_config();
    let active_ui = cfg_ui.unwrap_or_else(|| cur_theme.ui_font.clone());
    let active_mono = cfg_mono.unwrap_or_else(|| cur_theme.mono_font.clone());
    let active_size = cfg_size.unwrap_or(cur_theme.mono_font_size as f64);

    if let Some(pos) = font_names.iter().position(|f| f == &active_ui) {
        ui_font_dropdown.set_selected(pos as u32);
    }
    if let Some(pos) = font_names.iter().position(|f| f == &active_mono) {
        mono_font_dropdown.set_selected(pos as u32);
    }

    let size_label = nm_field_label("Terminal Font Size (10..18 pt)");
    let size_spin = SpinButton::with_range(10.0, 18.0, 1.0);
    size_spin.set_value(active_size);

    let s_ctx_fonts = s_ctx.clone();
    let font_names_ui = font_names.clone();
    let font_names_mono = font_names;

    let ui_dd_weak = ui_font_dropdown.downgrade();
    let mono_dd_weak = mono_font_dropdown.downgrade();
    let spin_weak = size_spin.downgrade();

    let trigger_font_update = move || {
        let (ui_idx, mono_idx, size) = match (
            ui_dd_weak.upgrade(),
            mono_dd_weak.upgrade(),
            spin_weak.upgrade(),
        ) {
            (Some(u), Some(m), Some(s)) => {
                (u.selected() as usize, m.selected() as usize, s.value())
            }
            _ => return,
        };

        let ui_name = font_names_ui
            .get(ui_idx)
            .cloned()
            .unwrap_or_else(|| "Inter".to_string());
        let mono_name = font_names_mono
            .get(mono_idx)
            .cloned()
            .unwrap_or_else(|| "JetBrains Mono".to_string());

        let _ = set_appearance_config(&ui_name, &mono_name, size);
        (s_ctx_fonts.on_appearance_updated)(&ui_name, &mono_name, size);
    };

    let tf1 = Rc::new(trigger_font_update);
    let tf2 = tf1.clone();
    let tf3 = tf1.clone();

    ui_font_dropdown.connect_selected_notify(move |_| {
        tf1();
    });
    mono_font_dropdown.connect_selected_notify(move |_| {
        tf2();
    });
    size_spin.connect_value_changed(move |_| {
        tf3();
    });

    let card2 = nm_card([
        card2_header.upcast::<gtk4::Widget>(),
        ui_font_label.upcast::<gtk4::Widget>(),
        ui_font_dropdown.upcast::<gtk4::Widget>(),
        mono_font_label.upcast::<gtk4::Widget>(),
        mono_font_dropdown.upcast::<gtk4::Widget>(),
        size_label.upcast::<gtk4::Widget>(),
        size_spin.upcast::<gtk4::Widget>(),
    ]);
    page_cards.push(card2.upcast::<gtk4::Widget>());

    // ==========================================
    // CARD 3: NAVIGATION TABS
    // ==========================================
    let card3_header = nm_section_header("NAVIGATION TABS");
    let card3_desc = nm_field_label("Toggle discovered sidebar tabs");

    let mut tab_widgets: Vec<gtk4::Widget> = vec![
        card3_header.upcast::<gtk4::Widget>(),
        card3_desc.upcast::<gtk4::Widget>(),
    ];

    let disabled_plugins = get_disabled_plugins();

    for spec in crate::plugins::ALL_DISCOVERED_SPECS {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_hexpand(true);

        let icon = nm_icon(spec.icon, Tone::Secondary, 18);
        row.append(icon.widget());

        let label = nm_data_label(spec.title);
        label.set_tone(Tone::Primary);
        row.append(label.widget());

        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        row.append(&spacer);

        let switch = Switch::new();
        let is_disabled = disabled_plugins.contains(&spec.id.to_string());
        switch.set_active(!is_disabled);

        let spec_id = spec.id.to_string();
        let s_ctx_tab = s_ctx.clone();
        switch.connect_state_set(move |_, state| {
            let disabled = !state;
            let _ = set_plugin_disabled(&spec_id, disabled);
            if let Some(ref nr) = *s_ctx_tab.nav_rail.borrow() {
                nr.set_tab_visible(&spec_id, state);
            }
            glib::Propagation::Proceed
        });

        row.append(&switch);
        tab_widgets.push(row.upcast::<gtk4::Widget>());
    }

    let card3 = nm_card(tab_widgets);
    page_cards.push(card3.upcast::<gtk4::Widget>());

    // ==========================================
    // CARDS 4+: PER-PLUGIN SETTINGS CARDS
    // ==========================================
    for spec in crate::plugins::ALL_DISCOVERED_SPECS {
        if spec.settings.is_empty() && spec.settings_custom.is_none() {
            continue;
        }

        let p_ctx = PluginContext::new(
            spec.id,
            s_ctx.theme.clone(),
            s_ctx.tmux.clone(),
            s_ctx.systemd.clone(),
            s_ctx.kwin.clone(),
        );

        let mut card_widgets: Vec<gtk4::Widget> = Vec::new();
        let header = nm_section_header(&format!("{} SETTINGS", spec.title.to_uppercase()));
        card_widgets.push(header.upcast::<gtk4::Widget>());

        // Custom settings builder if defined
        if let Some(custom_fn) = spec.settings_custom {
            let custom_widget = custom_fn(&p_ctx);
            card_widgets.push(custom_widget);
        }

        // Standard SettingSpecs
        for setting in spec.settings {
            let label = nm_field_label(setting.label);
            card_widgets.push(label.upcast::<gtk4::Widget>());

            let key = setting.key;
            let p_ctx_set = p_ctx.clone();

            match setting.kind {
                SettingKind::Text => {
                    let entry = Entry::new();
                    if let Some(val) = p_ctx.setting_str(key) {
                        entry.set_text(&val);
                    }
                    entry.set_hexpand(true);
                    let k_str = key.to_string();
                    entry.connect_changed(move |e| {
                        let text = e.text().to_string();
                        let _ = p_ctx_set.set_setting(&k_str, toml::Value::String(text));
                    });
                    card_widgets.push(entry.upcast::<gtk4::Widget>());
                }
                SettingKind::Secret => {
                    let hbox = GtkBox::new(Orientation::Horizontal, 6);
                    hbox.set_hexpand(true);

                    let entry = Entry::new();
                    entry.set_visibility(false);
                    entry.set_hexpand(true);
                    if let Some(val) = p_ctx.secret_get_blocking(key) {
                        entry.set_text(&val);
                    }

                    let save_btn = nm_icon_button("save", "Save Secret");
                    let k_str = key.to_string();
                    let entry_weak = entry.downgrade();
                    let p_ctx_sec = p_ctx_set.clone();
                    save_btn.on_click(move || {
                        if let Some(e) = entry_weak.upgrade() {
                            let val = e.text().to_string();
                            let _ = p_ctx_sec.secret_set_blocking(&k_str, &val);
                        }
                    });

                    hbox.append(&entry);
                    hbox.append(save_btn.widget());
                    card_widgets.push(hbox.upcast::<gtk4::Widget>());
                }
                SettingKind::Bool => {
                    let switch = Switch::new();
                    switch.set_active(p_ctx.setting_bool(key).unwrap_or(false));
                    let k_str = key.to_string();
                    switch.connect_state_set(move |_, state| {
                        let _ = p_ctx_set.set_setting(&k_str, toml::Value::Boolean(state));
                        glib::Propagation::Proceed
                    });
                    card_widgets.push(switch.upcast::<gtk4::Widget>());
                }
                SettingKind::Int => {
                    let spin = SpinButton::with_range(0.0, 86400.0, 1.0);
                    spin.set_value(p_ctx.setting_int(key).unwrap_or(0) as f64);
                    let k_str = key.to_string();
                    spin.connect_value_changed(move |s| {
                        let val = s.value() as i64;
                        let _ = p_ctx_set.set_setting(&k_str, toml::Value::Integer(val));
                    });
                    card_widgets.push(spin.upcast::<gtk4::Widget>());
                }
                SettingKind::Choice(choices) => {
                    let str_list = StringList::new(choices);
                    let dd = DropDown::new(Some(str_list), None::<gtk4::Expression>);
                    let cur_choice = p_ctx.setting_str(key);
                    if let Some(ref c) = cur_choice {
                        if let Some(pos) = choices.iter().position(|ch| ch == c) {
                            dd.set_selected(pos as u32);
                        }
                    }
                    let k_str = key.to_string();
                    let choices_vec: Vec<&'static str> = choices.to_vec();
                    dd.connect_selected_notify(move |d| {
                        let idx = d.selected() as usize;
                        if let Some(choice) = choices_vec.get(idx) {
                            let _ = p_ctx_set
                                .set_setting(&k_str, toml::Value::String((*choice).to_string()));
                        }
                    });
                    card_widgets.push(dd.upcast::<gtk4::Widget>());
                }
                SettingKind::Path => {
                    let entry = Entry::new();
                    if let Some(val) = p_ctx.setting_str(key) {
                        entry.set_text(&val);
                    }
                    entry.set_hexpand(true);
                    let k_str = key.to_string();
                    entry.connect_changed(move |e| {
                        let text = e.text().to_string();
                        let _ = p_ctx_set.set_setting(&k_str, toml::Value::String(text));
                    });
                    card_widgets.push(entry.upcast::<gtk4::Widget>());
                }
            }
        }

        let plugin_card = nm_card(card_widgets);
        page_cards.push(plugin_card.upcast::<gtk4::Widget>());
    }

    let page = nm_page(page_cards);
    wrap_in_plugin_container(&page.upcast::<gtk4::Widget>(), false)
}

fn is_valid_hex(s: &str) -> bool {
    let trimmed = s.trim();
    if !trimmed.starts_with('#') {
        return false;
    }
    let hex_part = &trimmed[1..];
    hex_part.len() == 6 && hex_part.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_page_build_and_width() {
        if gtk4::init().is_err() {
            eprintln!("Skipping settings page test: headless without display");
            return;
        }

        let theme_cell = Rc::new(RefCell::new(ThemePalette::default()));
        let nav_rail_cell = Rc::new(RefCell::new(None));
        let registry = Rc::new(RefCell::new(PluginRegistry::new()));

        let s_ctx = Rc::new(SettingsPageContext {
            theme: theme_cell,
            nav_rail: nav_rail_cell,
            registry,
            on_theme_updated: Rc::new(|_| {}),
            on_appearance_updated: Rc::new(|_, _, _| {}),
            tmux: nevermore_core::TmuxBackend::new("nevermore"),
            systemd: nevermore_core::SystemdTaskBackend::new("nevermore"),
            kwin: nevermore_core::KWinAdapter::new(),
        });

        let scrolled = build_settings_page(s_ctx);
        assert_eq!(scrolled.min_content_width(), 340);
        assert_eq!(scrolled.max_content_width(), 340);
        assert!(!scrolled.propagates_natural_width());
    }

    #[test]
    fn test_is_valid_hex() {
        assert!(is_valid_hex("#18181b"));
        assert!(is_valid_hex("#ffffff"));
        assert!(is_valid_hex("#89B4FA"));
        assert!(!is_valid_hex("18181b"));
        assert!(!is_valid_hex("#18181"));
        assert!(!is_valid_hex("#18181bg"));
    }
}
