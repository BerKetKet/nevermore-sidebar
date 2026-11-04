use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "template",
    "Template Plugin",
    "terminal",
    99,
    "Authoritative reference implementation for Nevermore plugins",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Data-only status label
    let status = nm_data_label("IDLE");
    status.set_tone(Tone::Muted);

    // 2. Card with action button
    let btn = nm_icon_button("check", "Execute action");
    let status_clone = status.clone();
    btn.on_click(move || {
        status_clone.set("ACTIVE");
        status_clone.set_tone(Tone::Accent);
    });

    let card = nm_card([
        status.widget().clone().upcast::<gtk4::Widget>(),
        btn.widget().clone().upcast::<gtk4::Widget>(),
    ]);

    // 3. Periodic timer (automatically paused when tab is hidden)
    let status_timer = status;
    ctx.every(Duration::from_secs(10), move || {
        status_timer.set("PING");
    });

    let page = nm_page([card.upcast::<gtk4::Widget>()]);
    page.upcast::<gtk4::Widget>()
}
