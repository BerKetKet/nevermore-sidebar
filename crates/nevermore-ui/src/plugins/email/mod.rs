use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "email",
    "Email",
    "email",
    9,
    "Email inbox overview and unread count",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let unread_badge = nm_data_label("3 UNREAD");
    unread_badge.set_tone(Tone::Accent);

    let refresh_btn = nm_icon_button("rotate-cw", "Refresh inbox");
    let unread_clone = unread_badge.clone();
    refresh_btn.on_click(move || {
        unread_clone.set("SYNCED");
    });

    let toolbar = nm_toolbar(
        [unread_badge.widget().clone().upcast::<gtk4::Widget>()],
        [refresh_btn.widget().clone().upcast::<gtk4::Widget>()],
    );

    let r1 = nm_list_row("mail", "GITHUB", Some("NOTIF"), []);
    let r2 = nm_list_row("mail", "SYSTEM", Some("OK"), []);
    let r3 = nm_list_row("mail", "SECURITY", Some("INFO"), []);
    let inbox_list = nm_list();
    inbox_list.append(&r1);
    inbox_list.append(&r2);
    inbox_list.append(&r3);
    let inbox_card = nm_card([inbox_list.upcast::<gtk4::Widget>()]);

    let unread_timer = unread_badge;
    ctx.every(Duration::from_secs(60), move || {
        unread_timer.set("3 UNREAD");
    });

    let page = nm_page([
        toolbar.upcast::<gtk4::Widget>(),
        inbox_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
