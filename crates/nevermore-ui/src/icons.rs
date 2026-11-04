use gtk4::cairo;
use gtk4::gdk;
use gtk4::gdk::prelude::TextureExtManual;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::sdk::types::{IconSource, Tone};
use nevermore_core::theme::ThemePalette;

// Auto-generated include from build.rs containing all 57 embedded SVGs
include!(concat!(env!("OUT_DIR"), "/icons_gen.rs"));

const SVG_CIRCLE_HELP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/></svg>"#;

thread_local! {
    static SURFACE_CACHE: RefCell<HashMap<(u64, i32, i32), cairo::ImageSurface>> = RefCell::new(HashMap::new());
    static TEXTURE_CACHE: RefCell<HashMap<(String, String, i32), gdk::Texture>> = RefCell::new(HashMap::new());
}

/// Resolves an IconSource into its raw SVG XML string.
pub fn resolve_svg(source: &IconSource) -> String {
    match source {
        IconSource::Svg(xml) => xml.clone(),
        IconSource::Named(name) => {
            let clean = name.strip_suffix(".svg").unwrap_or(name);
            if let Some(svg) = get_embedded_icon_svg(clean) {
                return svg.to_string();
            }
            // Normalization mappings for legacy/alternate naming
            let alt = match clean {
                "close" => Some("x"),
                "trash" => Some("trash-2"),
                "ram" => Some("memory-stick"),
                "volume" => Some("volume-2"),
                "mute" => Some("volume-x"),
                "attach" => Some("corner-up-left"),
                "sparkles" => Some("gemini"),
                "arrow-right" => Some("skip-forward"),
                _ => None,
            };
            if let Some(a) = alt {
                if let Some(svg) = get_embedded_icon_svg(a) {
                    return svg.to_string();
                }
            }
            eprintln!(
                "[nevermore-icons] warning: unknown icon '{}', falling back to circle-help",
                name
            );
            get_embedded_icon_svg("circle-help")
                .unwrap_or(SVG_CIRCLE_HELP)
                .to_string()
        }
    }
}

/// Checks whether an icon name exists in the embedded bundle.
pub fn has_icon(name: &str) -> bool {
    let clean = name.strip_suffix(".svg").unwrap_or(name);
    get_embedded_icon_svg(clean).is_some()
        || matches!(
            clean,
            "close" | "trash" | "ram" | "volume" | "mute" | "attach" | "sparkles" | "arrow-right"
        )
}

/// Returns or caches a black-alpha Cairo image surface for an SVG at (size_px * scale).
fn get_or_render_surface(
    svg_content: &str,
    size_px: i32,
    scale: i32,
) -> Option<cairo::ImageSurface> {
    let target_px = size_px * scale;
    let mut hasher = DefaultHasher::new();
    svg_content.hash(&mut hasher);
    let hash = hasher.finish();
    let key = (hash, size_px, scale);

    SURFACE_CACHE.with(|cache| {
        if let Some(surf) = cache.borrow().get(&key) {
            return Some(surf.clone());
        }

        let mut svg_black = svg_content.replace("currentColor", "#000000");
        if !svg_black.contains("width=") {
            svg_black = svg_black.replace(
                "<svg",
                &format!("<svg width=\"{}\" height=\"{}\"", target_px, target_px),
            );
        } else {
            svg_black = svg_black.replace("width=\"24\"", &format!("width=\"{}\"", target_px));
            svg_black = svg_black.replace("height=\"24\"", &format!("height=\"{}\"", target_px));
        }

        let bytes = glib::Bytes::from(svg_black.as_bytes());
        let texture = match gdk::Texture::from_bytes(&bytes) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[nevermore-icons] failed to parse SVG texture: {}", e);
                return None;
            }
        };

        let w = texture.width();
        let h = texture.height();
        let stride = w * 4;
        let mut data = vec![0u8; (stride * h) as usize];
        texture.download(&mut data, stride as usize);

        let surface =
            match cairo::ImageSurface::create_for_data(data, cairo::Format::ARgb32, w, h, stride) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[nevermore-icons] failed to create cairo surface: {:?}", e);
                    return None;
                }
            };

        cache.borrow_mut().insert(key, surface.clone());
        Some(surface)
    })
}

/// Helper that configures the Cairo draw function on a DrawingArea for an SVG.
fn setup_draw_func(area: &gtk4::DrawingArea, svg_str: String, size_px: i32) {
    area.set_draw_func(move |widget, cr, width, height| {
        let scale = widget.scale_factor();
        if let Some(surface) = get_or_render_surface(&svg_str, size_px, scale) {
            let color = widget.color();
            cr.set_source_rgba(
                color.red() as f64,
                color.green() as f64,
                color.blue() as f64,
                color.alpha() as f64,
            );
            let surf_w = surface.width() as f64 / scale as f64;
            let surf_h = surface.height() as f64 / scale as f64;
            let x = (width as f64 - surf_w) / 2.0;
            let y = (height as f64 - surf_h) / 2.0;
            let _ = cr.save();
            cr.scale(1.0 / scale as f64, 1.0 / scale as f64);
            let _ = cr.mask_surface(&surface, x * scale as f64, y * scale as f64);
            let _ = cr.restore();
        }
    });
}

/// Creates a CSS-tinted `gtk4::DrawingArea` for an icon.
pub fn create_icon_drawing_area(
    icon: impl Into<IconSource>,
    tone: Tone,
    size_px: i32,
) -> gtk4::DrawingArea {
    let area = gtk4::DrawingArea::new();
    area.set_content_width(size_px);
    area.set_content_height(size_px);
    area.add_css_class("nm-icon");
    area.add_css_class(tone.css_class());

    let source = icon.into();
    let svg_str = resolve_svg(&source);
    setup_draw_func(&area, svg_str, size_px);

    area
}

/// Wrapper for a CSS-tinted icon backed by a `gtk4::DrawingArea`.
#[derive(Clone)]
pub struct TintedIcon {
    area: gtk4::DrawingArea,
    source: Rc<RefCell<IconSource>>,
    tone: Rc<Cell<Tone>>,
    size_px: i32,
}

impl TintedIcon {
    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.area
    }

    pub fn set_icon(&self, icon: impl Into<IconSource>) {
        let new_source = icon.into();
        *self.source.borrow_mut() = new_source.clone();
        let svg_str = resolve_svg(&new_source);
        setup_draw_func(&self.area, svg_str, self.size_px);
        self.area.queue_draw();
    }

    pub fn set_tone(&self, tone: Tone) {
        let old_tone = self.tone.get();
        self.area.remove_css_class(old_tone.css_class());
        self.area.add_css_class(tone.css_class());
        self.tone.set(tone);
        self.area.queue_draw();
    }

    pub fn set_on_accent(&self, on_accent: bool) {
        if on_accent {
            self.area.add_css_class("on-accent");
        } else {
            self.area.remove_css_class("on-accent");
        }
        self.area.queue_draw();
    }
}

impl std::ops::Deref for TintedIcon {
    type Target = gtk4::DrawingArea;
    fn deref(&self) -> &Self::Target {
        &self.area
    }
}

/// Constructs a CSS-driven TintedIcon whose draw func reads `widget.color()` automatically.
pub fn nm_icon(icon: impl Into<IconSource>, tone: Tone, size_px: i32) -> TintedIcon {
    let source = icon.into();
    let area = create_icon_drawing_area(source.clone(), tone, size_px);

    TintedIcon {
        area,
        source: Rc::new(RefCell::new(source)),
        tone: Rc::new(Cell::new(tone)),
        size_px,
    }
}

/// Sets the icon of a button dynamically using a CSS-driven DrawingArea.
pub fn set_button_icon(btn: &gtk4::Button, name: &str, _color: &str, size_px: i32) {
    let area = create_icon_drawing_area(name, Tone::Secondary, size_px);
    btn.set_child(Some(&area));
}

/// Creates a standalone icon DrawingArea.
pub fn create_icon_widget(name: &str, _color: &str, size_px: i32) -> gtk4::DrawingArea {
    create_icon_drawing_area(name, Tone::Secondary, size_px)
}

/// No-op theme change hook retained for interface stability; CSS handles dynamic recoloring.
pub fn on_theme_changed(_palette: &ThemePalette) {
    // Intentionally empty: CSS custom properties and .nm-tone-* classes trigger redraws automatically.
}

/// Generates a `gdk::Texture` dynamically colored with `hex_color` at `size_px` (legacy helper).
pub fn icon_texture(name: &str, hex_color: &str, size_px: i32) -> gdk::Texture {
    let key = (name.to_string(), hex_color.to_string(), size_px);
    if let Some(tex) = TEXTURE_CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return tex;
    }

    let source = IconSource::Named(name.to_string());
    let template = resolve_svg(&source);
    let mut colored = template.replace("currentColor", hex_color);
    colored = colored.replace("width=\"24\"", &format!("width=\"{}\"", size_px));
    colored = colored.replace("height=\"24\"", &format!("height=\"{}\"", size_px));

    let bytes = glib::Bytes::from(colored.as_bytes());
    let texture = gdk::Texture::from_bytes(&bytes).unwrap_or_else(|_| {
        let fallback_colored = SVG_CIRCLE_HELP.replace("currentColor", hex_color);
        let fb_bytes = glib::Bytes::from(fallback_colored.as_bytes());
        gdk::Texture::from_bytes(&fb_bytes).expect("Failed to parse fallback circle-help SVG")
    });

    TEXTURE_CACHE.with(|c| {
        c.borrow_mut().insert(key, texture.clone());
    });

    texture
}
