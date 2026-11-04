use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "vpn",
    "VPN Status",
    "vpn",
    8,
    "VPN Wireguard connection monitor and toggle",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let status_dot = nm_status_dot(Tone::Success);
    let status_label = nm_data_label("CONNECTED");
    status_label.set_tone(Tone::Success);

    let toggle = nm_toggle_icon_button("shield", "shield-off", "Toggle VPN connection");
    let status_dot_clone = status_dot.clone();
    let status_label_clone = status_label.clone();
    toggle.on_toggle(move |active| {
        if active {
            status_dot_clone.set_tone(Tone::Success);
            status_label_clone.set("CONNECTED");
        } else {
            status_dot_clone.set_tone(Tone::Muted);
            status_label_clone.set("DISCONNECTED");
        }
    });

    let status_card = nm_card([
        status_dot.widget().clone().upcast::<gtk4::Widget>(),
        status_label.widget().clone().upcast::<gtk4::Widget>(),
        toggle.widget().clone().upcast::<gtk4::Widget>(),
    ]);

    let latency = nm_metric_row("clock", Tone::Accent);
    latency.set(0.18, "18 ms");
    let metrics_card = nm_card([latency.widget().clone().upcast::<gtk4::Widget>()]);

    let s1 = nm_list_row("globe", "DEFAULT", Some("ACTIVE"), []);
    let s2 = nm_list_row("globe", "FALLBACK", Some("STANDBY"), []);
    let server_list = nm_list();
    server_list.append(&s1);
    server_list.append(&s2);
    let server_card = nm_card([server_list.upcast::<gtk4::Widget>()]);

    ctx.every(Duration::from_secs(5), {
        let latency = latency.clone();
        move || {
            latency.set(0.20, "20 ms");
        }
    });

    let page = nm_page([
        status_card.upcast::<gtk4::Widget>(),
        metrics_card.upcast::<gtk4::Widget>(),
        server_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
