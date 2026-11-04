pub use crate::icons::{nm_icon, TintedIcon};
use crate::sdk::types::{IconSource, Tone};
use gtk4::accessible::Property;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use gtk4::{
    glib, Box as GtkBox, Button, DrawingArea, Entry, Label, ListBox, ListBoxRow, Orientation,
    ProgressBar,
};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::time::Duration;

/// 1. nm_page: 340px-safe vertical page container with standard padding and spacing.
pub fn nm_page(children: impl IntoIterator<Item = gtk4::Widget>) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 10);
    page.set_hexpand(true);
    page.set_margin_top(12);
    page.set_margin_bottom(12);
    page.set_margin_start(12);
    page.set_margin_end(12);
    page.add_css_class("nm-page");
    for child in children {
        page.append(&child);
    }
    page
}

/// 2. nm_card: Standardized secondary-background card container.
pub fn nm_card(children: impl IntoIterator<Item = gtk4::Widget>) -> GtkBox {
    let card = GtkBox::new(Orientation::Vertical, 8);
    card.add_css_class("nm-card");
    for child in children {
        card.append(&child);
    }
    card
}

/// 3. nm_toolbar: Horizontal bar with left widgets, expanding center spacer, and right widgets.
pub fn nm_toolbar(
    left_children: impl IntoIterator<Item = gtk4::Widget>,
    right_children: impl IntoIterator<Item = gtk4::Widget>,
) -> GtkBox {
    let toolbar = GtkBox::new(Orientation::Horizontal, 6);
    toolbar.add_css_class("nm-toolbar");
    for child in left_children {
        toolbar.append(&child);
    }
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    toolbar.append(&spacer);
    for child in right_children {
        toolbar.append(&child);
    }
    toolbar
}

// 4. TintedIcon & nm_icon: Self-tinting SVG icon tracking theme palette and tone (defined in crate::icons).

/// 5. IconButton & nm_icon_button: Accessible button with automatic tooltip and label.
#[derive(Clone)]
pub struct IconButton {
    btn: Button,
    icon: TintedIcon,
}

impl IconButton {
    pub fn widget(&self) -> &Button {
        &self.btn
    }

    pub fn icon(&self) -> &TintedIcon {
        &self.icon
    }

    pub fn on_click(&self, f: impl Fn() + 'static) {
        self.btn.connect_clicked(move |_| {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                f();
            }));
        });
    }
}

impl Deref for IconButton {
    type Target = Button;
    fn deref(&self) -> &Self::Target {
        &self.btn
    }
}

pub fn nm_icon_button(icon: impl Into<IconSource>, tooltip: &str) -> IconButton {
    let btn = Button::new();
    btn.set_tooltip_text(Some(tooltip));
    btn.update_property(&[Property::Label(tooltip)]);
    btn.add_css_class("icon-btn");

    let tinted_icon = nm_icon(icon, Tone::Secondary, 18);
    btn.set_child(Some(tinted_icon.widget()));

    IconButton {
        btn,
        icon: tinted_icon,
    }
}

/// 6. ToggleIconButton & nm_toggle_icon_button: Toggling icon button with active accent style.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct ToggleIconButton {
    btn: Button,
    icon: TintedIcon,
    on_icon: IconSource,
    off_icon: IconSource,
    is_active: Rc<Cell<bool>>,
    on_toggle_cb: Rc<RefCell<Option<Box<dyn Fn(bool)>>>>,
}

impl ToggleIconButton {
    pub fn widget(&self) -> &Button {
        &self.btn
    }

    pub fn is_active(&self) -> bool {
        self.is_active.get()
    }

    pub fn set_active_silent(&self, active: bool) {
        self.is_active.set(active);
        self.update_visual(active);
    }

    pub fn on_toggle(&self, f: impl Fn(bool) + 'static) {
        *self.on_toggle_cb.borrow_mut() = Some(Box::new(f));
    }

    fn update_visual(&self, active: bool) {
        if active {
            self.btn.add_css_class("btn-active");
            self.btn.add_css_class("nm-btn-primary");
            self.icon.set_icon(self.on_icon.clone());
            self.icon.set_on_accent(true);
            self.icon.set_tone(Tone::Accent);
        } else {
            self.btn.remove_css_class("btn-active");
            self.btn.remove_css_class("nm-btn-primary");
            self.icon.set_icon(self.off_icon.clone());
            self.icon.set_on_accent(false);
            self.icon.set_tone(Tone::Secondary);
        }
    }
}

impl Deref for ToggleIconButton {
    type Target = Button;
    fn deref(&self) -> &Self::Target {
        &self.btn
    }
}

pub fn nm_toggle_icon_button(
    on_icon: impl Into<IconSource>,
    off_icon: impl Into<IconSource>,
    tooltip: &str,
) -> ToggleIconButton {
    let on_src = on_icon.into();
    let off_src = off_icon.into();

    let btn = Button::new();
    btn.set_tooltip_text(Some(tooltip));
    btn.update_property(&[Property::Label(tooltip)]);
    btn.add_css_class("icon-btn");

    let tinted_icon = nm_icon(off_src.clone(), Tone::Secondary, 18);
    btn.set_child(Some(tinted_icon.widget()));

    let is_active = Rc::new(Cell::new(false));
    let on_toggle_cb = Rc::new(RefCell::new(None));

    let toggle = ToggleIconButton {
        btn: btn.clone(),
        icon: tinted_icon,
        on_icon: on_src,
        off_icon: off_src,
        is_active: is_active.clone(),
        on_toggle_cb: on_toggle_cb.clone(),
    };

    let toggle_clone = toggle.clone();
    btn.connect_clicked(move |_| {
        let new_state = !is_active.get();
        is_active.set(new_state);
        toggle_clone.update_visual(new_state);
        if let Some(ref cb) = *on_toggle_cb.borrow() {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                cb(new_state);
            }));
        }
    });

    toggle
}

/// 7. DataLabel & nm_data_label: Safe data-only label with .nm-data class and tone colors.
#[derive(Clone)]
pub struct DataLabel {
    label: Label,
}

impl DataLabel {
    pub fn widget(&self) -> &Label {
        &self.label
    }

    pub fn set(&self, text: &str) {
        self.label.set_text(text);
    }

    pub fn set_tone(&self, tone: Tone) {
        self.label.remove_css_class("nm-tone-primary");
        self.label.remove_css_class("nm-tone-secondary");
        self.label.remove_css_class("nm-tone-muted");
        self.label.remove_css_class("nm-tone-accent");
        self.label.remove_css_class("nm-tone-danger");
        self.label.remove_css_class("nm-tone-success");
        self.label.remove_css_class("nm-tone-warning");

        match tone {
            Tone::Primary => self.label.add_css_class("nm-tone-primary"),
            Tone::Secondary => self.label.add_css_class("nm-tone-secondary"),
            Tone::Muted => self.label.add_css_class("nm-tone-muted"),
            Tone::Accent => self.label.add_css_class("nm-tone-accent"),
            Tone::Danger => self.label.add_css_class("nm-tone-danger"),
            Tone::Success => self.label.add_css_class("nm-tone-success"),
            Tone::Warning => self.label.add_css_class("nm-tone-warning"),
        }
    }
}

impl Deref for DataLabel {
    type Target = Label;
    fn deref(&self) -> &Self::Target {
        &self.label
    }
}

pub fn nm_data_label(initial_text: &str) -> DataLabel {
    let label = Label::new(Some(initial_text));
    label.add_css_class("nm-data");
    label.set_ellipsize(EllipsizeMode::End);
    label.set_xalign(0.0);
    DataLabel { label }
}

/// 8. MetricRow & nm_metric_row: Icon + Progress Bar + Value label.
#[derive(Clone)]
pub struct MetricRow {
    container: GtkBox,
    icon: TintedIcon,
    progress: ProgressBar,
    label: DataLabel,
}

impl MetricRow {
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn icon(&self) -> &TintedIcon {
        &self.icon
    }

    pub fn set(&self, fraction_0_to_1: f64, text: &str) {
        self.progress.set_fraction(fraction_0_to_1.clamp(0.0, 1.0));
        self.label.set(text);
    }
}

impl Deref for MetricRow {
    type Target = GtkBox;
    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

pub fn nm_metric_row(icon: impl Into<IconSource>, tone: Tone) -> MetricRow {
    let container = GtkBox::new(Orientation::Horizontal, 8);
    container.set_hexpand(true);

    let tinted_icon = nm_icon(icon, tone, 16);
    container.append(tinted_icon.widget());

    let progress = ProgressBar::new();
    progress.set_hexpand(true);
    progress.set_valign(gtk4::Align::Center);
    container.append(&progress);

    let label = nm_data_label("");
    label.set_tone(tone);
    label.set_width_chars(5);
    container.append(label.widget());

    MetricRow {
        container,
        icon: tinted_icon,
        progress,
        label,
    }
}

/// 9. nm_list & nm_list_row: Data-driven list view rows.
pub fn nm_list() -> ListBox {
    let list = ListBox::new();
    list.add_css_class("nm-list");
    list.set_selection_mode(gtk4::SelectionMode::None);
    list
}

pub fn nm_list_row(
    icon: impl Into<IconSource>,
    primary_data: &str,
    secondary_data: Option<&str>,
    action_widgets: impl IntoIterator<Item = gtk4::Widget>,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_top(6);
    hbox.set_margin_bottom(6);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);

    let tinted_icon = nm_icon(icon, Tone::Secondary, 18);
    hbox.append(tinted_icon.widget());

    let vbox = GtkBox::new(Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let p_label = nm_data_label(primary_data);
    p_label.set_tone(Tone::Primary);
    vbox.append(p_label.widget());

    if let Some(sec) = secondary_data {
        let s_label = nm_data_label(sec);
        s_label.set_tone(Tone::Muted);
        vbox.append(s_label.widget());
    }
    hbox.append(&vbox);

    for action in action_widgets {
        hbox.append(&action);
    }

    row.set_child(Some(&hbox));
    row
}

/// 10. nm_empty_state: Centered dim 32px icon with zero text.
pub fn nm_empty_state(icon: impl Into<IconSource>, tooltip: &str) -> GtkBox {
    let box_widget = GtkBox::new(Orientation::Vertical, 0);
    box_widget.set_vexpand(true);
    box_widget.set_hexpand(true);
    box_widget.set_valign(gtk4::Align::Center);
    box_widget.set_halign(gtk4::Align::Center);
    box_widget.set_tooltip_text(Some(tooltip));

    let tinted_icon = nm_icon(icon, Tone::Muted, 32);
    box_widget.append(tinted_icon.widget());
    box_widget
}

/// 11. nm_search_entry: Tooltip-only search entry with clear button and 200ms debounce.
pub fn nm_search_entry(tooltip: &str, on_change_debounced: impl Fn(String) + 'static) -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 6);
    container.add_css_class("nm-search-entry");

    let s_icon = nm_icon("search", Tone::Secondary, 16);
    container.append(s_icon.widget());

    let entry = Entry::new();
    entry.set_hexpand(true);
    entry.set_tooltip_text(Some(tooltip));
    entry.update_property(&[Property::Label(tooltip)]);
    container.append(&entry);

    let clear_btn = nm_icon_button("x", "Clear search");
    container.append(clear_btn.widget());

    let entry_weak = entry.downgrade();
    clear_btn.on_click(move || {
        if let Some(e) = entry_weak.upgrade() {
            e.set_text("");
        }
    });

    let debounced_fn = Rc::new(on_change_debounced);
    let pending_timeout: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let debounced_clone = debounced_fn.clone();
    let pending_clone = pending_timeout.clone();
    entry.connect_changed(move |e| {
        if let Some(source) = pending_clone.borrow_mut().take() {
            source.remove();
        }
        let text = e.text().to_string();
        let deb_cb = debounced_clone.clone();
        let timeout_handle = pending_clone.clone();

        let source = glib::timeout_add_local_once(Duration::from_millis(200), move || {
            *timeout_handle.borrow_mut() = None;
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                deb_cb(text);
            }));
        });
        *pending_clone.borrow_mut() = Some(source);
    });

    container
}

/// 12. StatusDot & nm_status_dot: 8x8 circular status indicator.
#[derive(Clone)]
pub struct StatusDot {
    area: DrawingArea,
    tone: Rc<Cell<Tone>>,
}

impl StatusDot {
    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }

    pub fn set_tone(&self, tone: Tone) {
        let old = self.tone.get();
        self.area.remove_css_class(old.css_class());
        self.area.add_css_class(tone.css_class());
        self.tone.set(tone);
        self.area.queue_draw();
    }
}

pub fn nm_status_dot(tone: Tone) -> StatusDot {
    let area = DrawingArea::new();
    area.set_size_request(8, 8);
    area.add_css_class("nm-status-dot");
    area.add_css_class(tone.css_class());
    let tone_cell = Rc::new(Cell::new(tone));

    area.set_draw_func(move |widget, cr, width, height| {
        let rgba = widget.color();
        cr.set_source_rgba(
            rgba.red() as f64,
            rgba.green() as f64,
            rgba.blue() as f64,
            rgba.alpha() as f64,
        );
        let radius = (width.min(height) as f64) / 2.0;
        cr.arc(
            width as f64 / 2.0,
            height as f64 / 2.0,
            radius,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        let _ = cr.fill();
    });

    StatusDot {
        area,
        tone: tone_cell,
    }
}

/// nm_segmented: Icon-only segmented control.
pub fn nm_segmented(
    icon_tooltip_pairs: Vec<(impl Into<IconSource>, &str)>,
    on_select: impl Fn(usize) + 'static,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 2);
    container.add_css_class("nm-segmented");

    let on_select = Rc::new(on_select);
    let buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    for (idx, (icon, tooltip)) in icon_tooltip_pairs.into_iter().enumerate() {
        let btn = nm_icon_button(icon, tooltip);
        let on_select_clone = on_select.clone();
        let buttons_clone = buttons.clone();

        btn.on_click(move || {
            for (i, b) in buttons_clone.borrow().iter().enumerate() {
                if i == idx {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
            on_select_clone(idx);
        });

        if idx == 0 {
            btn.widget().add_css_class("active");
        }
        buttons.borrow_mut().push(btn.widget().clone());
        container.append(btn.widget());
    }

    container
}
