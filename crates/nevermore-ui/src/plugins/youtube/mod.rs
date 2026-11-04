use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "youtube",
    "YouTube",
    "youtube",
    7,
    "YouTube search and playback queue",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let search_bar = nm_search_entry("Search YouTube", |_query| {});

    let r1 = nm_list_row("youtube", "STREAM", Some("LIVE"), []);
    let r2 = nm_list_row("play", "PLAYLIST 01", Some("READY"), []);
    let queue_list = nm_list();
    queue_list.append(&r1);
    queue_list.append(&r2);
    let queue_card = nm_card([queue_list.upcast::<gtk4::Widget>()]);

    let link_btn = nm_icon_button("external-link", "Open YouTube");
    link_btn.on_click(|| {
        let cmd = Cmd::new("xdg-open").arg("https://youtube.com");
        std::thread::spawn(move || {
            let _ = cmd.execute_blocking();
        });
    });
    let action_card = nm_card([link_btn.widget().clone().upcast::<gtk4::Widget>()]);

    ctx.every(Duration::from_secs(30), || {});

    let page = nm_page([
        search_bar.upcast::<gtk4::Widget>(),
        queue_card.upcast::<gtk4::Widget>(),
        action_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
