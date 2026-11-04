use gtk4::gdk;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

// Bundled 24x24 Lucide SVG templates (stroke="currentColor")
const SVG_TERMINAL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>"#;
const SVG_WIDGETS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="7" height="9" x="3" y="3" rx="1"/><rect width="7" height="5" x="14" y="3" rx="1"/><rect width="7" height="9" x="14" y="12" rx="1"/><rect width="7" height="5" x="3" y="16" rx="1"/></svg>"#;
const SVG_NOTIFICATIONS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>"#;
const SVG_NOTEPAD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><line x1="10" y1="9" x2="8" y2="9"/></svg>"#;
const SVG_MEDIA: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>"#;
const SVG_GEMINI: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"/><path d="M5 3v4"/><path d="M19 17v4"/><path d="M3 5h4"/><path d="M17 19h4"/></svg>"#;
const SVG_YOUTUBE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="15" x="2" y="4" rx="3"/><polygon points="10 8 16 11.5 10 15 10 8"/></svg>"#;
const SVG_VPN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#;
const SVG_EMAIL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/></svg>"#;
const SVG_PUZZLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19.439 7.85c0-1.571-.802-2.85-1.793-2.85s-1.793 1.279-1.793 2.85H12V3.561C12 2.7 10.72 2 9.15 2S6.3 2.7 6.3 3.561V7.85H2v3.85c1.571 0 2.85.802 2.85 1.793s-1.279 1.793-2.85 1.793V19h4.3v-4.289c0-.861 1.28-1.561 2.85-1.561s2.85.7 2.85 1.561V19h4.289c0-1.571.802-2.85 1.793-2.85s1.793 1.279 1.793 2.85H22v-4.3c-1.571 0-2.85-.802-2.85-1.793s1.279-1.793 2.85-1.793V7.85h-2.561z"/></svg>"#;

// Action and status icons
const SVG_PIN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="17" x2="12" y2="22"/><path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"/></svg>"#;
const SVG_PIN_OFF: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="2" y1="2" x2="22" y2="22"/><line x1="12" y1="17" x2="12" y2="22"/><path d="M9 9v1.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V17h12"/><path d="M15 9.34V6h1a2 2 0 0 0 0-4H7.89"/></svg>"#;
const SVG_X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;
const SVG_BELL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>"#;
const SVG_BELL_OFF: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8.7 3A6 6 0 0 1 18 8a21.3 21.3 0 0 0 .6 5"/><path d="M17 17H3s3-2 3-9a4.67 4.67 0 0 1 .3-1.7"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/><line x1="2" y1="2" x2="22" y2="22"/></svg>"#;
const SVG_TRASH_2: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>"#;
const SVG_PLUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>"#;
const SVG_LAYERS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"/><path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"/></svg>"#;
const SVG_CPU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/></svg>"#;
const SVG_MEMORY_STICK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 19v-3"/><path d="M10 19v-3"/><path d="M14 19v-3"/><path d="M18 19v-2"/><rect x="2" y="5" width="20" height="11" rx="2"/></svg>"#;
const SVG_VOLUME_2: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/></svg>"#;
const SVG_VOLUME_X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><line x1="22" y1="9" x2="16" y2="15"/><line x1="16" y1="9" x2="22" y2="15"/></svg>"#;
const SVG_SKIP_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="19 20 9 12 19 4 19 20"/><line x1="5" y1="19" x2="5" y2="5"/></svg>"#;
const SVG_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="6 3 20 12 6 21 6 3"/></svg>"#;
const SVG_PAUSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="14" y="4" width="4" height="16" rx="1"/><rect x="6" y="4" width="4" height="16" rx="1"/></svg>"#;
const SVG_SKIP_FORWARD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 4 15 12 5 20 5 4"/><line x1="19" y1="5" x2="19" y2="19"/></svg>"#;
const SVG_MUSIC: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>"#;
const SVG_ERASER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 21-4.3-4.3c-1-1-1-2.5 0-3.4l9.6-9.6c1-1 2.5-1 3.4 0l5.6 5.6c1 1 1 2.5 0 3.4L13 21"/><path d="M22 21H7"/><path d="m5 11 9 9"/></svg>"#;
const SVG_INBOX: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/></svg>"#;
const SVG_CORNER_UP_LEFT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 14 4 9 9 4"/><path d="M20 20v-7a4 4 0 0 0-4-4H4"/></svg>"#;
const SVG_SEARCH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>"#;
const SVG_SHIELD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>"#;
const SVG_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>"#;
const SVG_ALERT_TRIANGLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#;
const SVG_ALERT_CIRCLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>"#;

thread_local! {
    static TEXTURE_CACHE: RefCell<HashMap<(String, String, i32), gdk::Texture>> = RefCell::new(HashMap::new());
}

pub fn get_svg_template(name: &str) -> &'static str {
    match name {
        "terminal" | "terminal.svg" => SVG_TERMINAL,
        "widgets" | "widgets.svg" => SVG_WIDGETS,
        "notifications" | "notifications.svg" => SVG_NOTIFICATIONS,
        "notepad" | "notepad.svg" => SVG_NOTEPAD,
        "media" | "media.svg" => SVG_MEDIA,
        "gemini" | "gemini.svg" => SVG_GEMINI,
        "youtube" | "youtube.svg" => SVG_YOUTUBE,
        "vpn" | "vpn.svg" => SVG_VPN,
        "email" | "email.svg" => SVG_EMAIL,

        "pin" => SVG_PIN,
        "pin-off" => SVG_PIN_OFF,
        "x" | "close" => SVG_X,
        "bell" => SVG_BELL,
        "bell-off" => SVG_BELL_OFF,
        "trash-2" | "trash" => SVG_TRASH_2,
        "plus" => SVG_PLUS,
        "layers" => SVG_LAYERS,
        "cpu" => SVG_CPU,
        "memory-stick" | "ram" => SVG_MEMORY_STICK,
        "volume-2" | "volume" => SVG_VOLUME_2,
        "volume-x" | "mute" => SVG_VOLUME_X,
        "skip-back" => SVG_SKIP_BACK,
        "play" => SVG_PLAY,
        "pause" => SVG_PAUSE,
        "skip-forward" => SVG_SKIP_FORWARD,
        "music" => SVG_MUSIC,
        "eraser" => SVG_ERASER,
        "inbox" => SVG_INBOX,
        "corner-up-left" | "attach" => SVG_CORNER_UP_LEFT,
        "search" => SVG_SEARCH,
        "shield" => SVG_SHIELD,
        "check" => SVG_CHECK,
        "alert-triangle" => SVG_ALERT_TRIANGLE,
        "alert-circle" => SVG_ALERT_CIRCLE,
        "mail" => SVG_EMAIL,
        "sparkles" => SVG_GEMINI,
        "clock" => SVG_MEDIA,
        "chevron-left" => SVG_SKIP_BACK,
        "chevron-right" => SVG_SKIP_FORWARD,
        "arrow-right" => SVG_SKIP_FORWARD,
        "rotate-cw" => SVG_WIDGETS,
        "shield-off" => SVG_SHIELD,
        "globe" => SVG_VPN,
        "external-link" => SVG_TERMINAL,

        _ => SVG_PUZZLE,
    }
}

pub fn has_icon(name: &str) -> bool {
    let clean = name.strip_suffix(".svg").unwrap_or(name);
    matches!(
        clean,
        "terminal"
            | "widgets"
            | "notifications"
            | "notepad"
            | "media"
            | "gemini"
            | "youtube"
            | "vpn"
            | "email"
            | "pin"
            | "pin-off"
            | "x"
            | "close"
            | "bell"
            | "bell-off"
            | "trash-2"
            | "trash"
            | "plus"
            | "layers"
            | "cpu"
            | "memory-stick"
            | "ram"
            | "volume-2"
            | "volume"
            | "volume-x"
            | "mute"
            | "skip-back"
            | "play"
            | "pause"
            | "skip-forward"
            | "music"
            | "eraser"
            | "inbox"
            | "corner-up-left"
            | "attach"
            | "search"
            | "shield"
            | "check"
            | "alert-triangle"
            | "alert-circle"
            | "mail"
            | "sparkles"
            | "clock"
            | "chevron-left"
            | "chevron-right"
            | "arrow-right"
            | "rotate-cw"
            | "shield-off"
            | "globe"
            | "external-link"
            | "puzzle"
    )
}

use crate::sdk::types::{IconSource, Tone};
use nevermore_core::theme::{contrast_text_color_for_hex, ThemePalette};

pub fn resolve_tone_color(tone: Tone, palette: &ThemePalette, on_accent: bool) -> String {
    if on_accent {
        return contrast_text_color_for_hex(&palette.accent_color).to_string();
    }
    match tone {
        Tone::Primary => palette.text_primary.clone(),
        Tone::Secondary => palette.text_secondary.clone(),
        Tone::Muted => palette.text_secondary.clone(),
        Tone::Accent => palette.accent_color.clone(),
        Tone::Danger => palette.danger_color.clone(),
        Tone::Success => palette.success_color.clone(),
        Tone::Warning => palette.warning_color.clone(),
    }
}

pub struct IconBinding {
    pub image: glib::WeakRef<gtk4::Image>,
    pub source: IconSource,
    pub tone: Tone,
    pub size_px: i32,
    pub on_accent: bool,
}

thread_local! {
    static ICON_BINDINGS: RefCell<Vec<IconBinding>> = const { RefCell::new(Vec::new()) };
    static CURRENT_PALETTE: RefCell<ThemePalette> = RefCell::new(ThemePalette::default());
}

pub fn get_current_palette() -> ThemePalette {
    CURRENT_PALETTE.with(|p| p.borrow().clone())
}

pub fn register_icon_binding(binding: IconBinding) {
    ICON_BINDINGS.with(|b| {
        b.borrow_mut().push(binding);
    });
}

pub fn on_theme_changed(palette: &ThemePalette) {
    CURRENT_PALETTE.with(|p| {
        *p.borrow_mut() = palette.clone();
    });

    ICON_BINDINGS.with(|b| {
        let mut bindings = b.borrow_mut();
        bindings.retain(|item| {
            if let Some(img) = item.image.upgrade() {
                let color = resolve_tone_color(item.tone, palette, item.on_accent);
                let tex = raw_icon_texture(&item.source, &color, item.size_px);
                img.set_paintable(Some(&tex));
                true
            } else {
                false
            }
        });
    });
}

pub fn raw_icon_texture(source: &IconSource, hex_color: &str, size_px: i32) -> gdk::Texture {
    match source {
        IconSource::Named(name) => icon_texture(name, hex_color, size_px),
        IconSource::Svg(xml) => {
            let key = (xml.clone(), hex_color.to_string(), size_px);
            if let Some(tex) = TEXTURE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
                return tex;
            }
            let mut colored = xml.replace("currentColor", hex_color);
            colored = colored.replace("width=\"24\"", &format!("width=\"{}\"", size_px));
            colored = colored.replace("height=\"24\"", &format!("height=\"{}\"", size_px));
            let bytes = glib::Bytes::from(colored.as_bytes());
            let texture = gdk::Texture::from_bytes(&bytes)
                .unwrap_or_else(|_| icon_texture("puzzle", hex_color, size_px));
            TEXTURE_CACHE.with(|c| {
                c.borrow_mut().insert(key, texture.clone());
            });
            texture
        }
    }
}

/// Loads a custom SVG string from a relative path or returns the bundled template (A7).
#[allow(dead_code)]
pub fn resolve_svg_content(name_or_path: &str, config_dir: Option<&Path>) -> String {
    if !name_or_path.starts_with('/') && !name_or_path.contains("..") {
        if let Some(base) = config_dir {
            let path = base.join(name_or_path);
            if path.exists() {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.len() <= 65536 {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            return content;
                        }
                    }
                }
            }
        }
    }
    get_svg_template(name_or_path).to_string()
}

/// Generates a `gdk::Texture` dynamically colored with `hex_color` at `size_px`.
pub fn icon_texture(name: &str, hex_color: &str, size_px: i32) -> gdk::Texture {
    let key = (name.to_string(), hex_color.to_string(), size_px);
    if let Some(tex) = TEXTURE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return tex;
    }

    let template = get_svg_template(name);
    let mut colored = template.replace("currentColor", hex_color);
    colored = colored.replace("width=\"24\"", &format!("width=\"{}\"", size_px));
    colored = colored.replace("height=\"24\"", &format!("height=\"{}\"", size_px));

    let bytes = glib::Bytes::from(colored.as_bytes());
    let texture = gdk::Texture::from_bytes(&bytes).unwrap_or_else(|_| {
        let fallback_colored = SVG_PUZZLE.replace("currentColor", hex_color);
        let fb_bytes = glib::Bytes::from(fallback_colored.as_bytes());
        gdk::Texture::from_bytes(&fb_bytes).expect("Failed to parse fallback puzzle SVG")
    });

    TEXTURE_CACHE.with(|c| {
        c.borrow_mut().insert(key, texture.clone());
    });

    texture
}

/// Sets the icon of a button dynamically with the specified Lucide icon name, color, and size.
pub fn set_button_icon(btn: &gtk4::Button, name: &str, hex_color: &str, size_px: i32) {
    let texture = icon_texture(name, hex_color, size_px);
    let img = gtk4::Image::from_paintable(Some(&texture));
    btn.set_child(Some(&img));
}

/// Creates a standalone `gtk4::Image` widget with the specified icon, color, and size.
pub fn create_icon_widget(name: &str, hex_color: &str, size_px: i32) -> gtk4::Image {
    let texture = icon_texture(name, hex_color, size_px);
    gtk4::Image::from_paintable(Some(&texture))
}
