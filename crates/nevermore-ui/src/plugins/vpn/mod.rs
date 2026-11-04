use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "vpn",
    "VPN Status",
    "vpn",
    8,
    "VPN Wireguard connection monitor and toggle",
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VpnProfile {
    pub name: String,
    pub uuid: String,
    pub conn_type: String,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub enum VpnMsg {
    WarpStatus { connected: bool, status_text: String },
    NmcliProfiles(Vec<VpnProfile>),
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Card 1: Cloudflare WARP
    let warp_dot = nm_status_dot(Tone::Muted);
    let warp_label = nm_data_label("DISCONNECTED");
    warp_label.set_tone(Tone::Muted);

    let warp_toggle = nm_toggle_icon_button("shield", "shield", "Connect or disconnect Cloudflare WARP");

    let warp_toolbar = nm_toolbar(
        [
            warp_dot.widget().clone().upcast::<gtk4::Widget>(),
            warp_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [warp_toggle.widget().clone().upcast::<gtk4::Widget>()],
    );
    let warp_card = nm_card([warp_toolbar.upcast::<gtk4::Widget>()]);

    // 2. Card 2: WireGuard / NetworkManager Profiles List
    let profile_list = nm_list();
    let initial_empty = nm_empty_state("vpn", "No WireGuard profiles configured");
    let initial_row = gtk4::ListBoxRow::new();
    initial_row.set_child(Some(&initial_empty));
    profile_list.append(&initial_row);

    let profile_card = nm_card([profile_list.clone().upcast::<gtk4::Widget>()]);

    // 3. Mailbox Handler
    let dot_for_msg = warp_dot;
    let label_for_msg = warp_label;
    let list_for_msg = profile_list;
    let ctx_profile_toggle = ctx.clone();

    let tx = ctx.mailbox(move |msg: VpnMsg| match msg {
        VpnMsg::WarpStatus { connected, status_text } => {
            if connected {
                dot_for_msg.set_tone(Tone::Success);
                label_for_msg.set_tone(Tone::Success);
            } else {
                dot_for_msg.set_tone(Tone::Muted);
                label_for_msg.set_tone(Tone::Muted);
            }
            label_for_msg.set(&status_text);
        }
        VpnMsg::NmcliProfiles(profiles) => {
            while let Some(child) = list_for_msg.first_child() {
                list_for_msg.remove(&child);
            }
            if profiles.is_empty() {
                let empty = nm_empty_state("vpn", "No WireGuard profiles configured");
                let row = gtk4::ListBoxRow::new();
                row.set_child(Some(&empty));
                list_for_msg.append(&row);
            } else {
                for prof in profiles {
                    let toggle_action = nm_toggle_icon_button(
                        "shield",
                        "shield",
                        "Toggle WireGuard profile activation",
                    );
                    let name_clone = prof.name.clone();
                    let ctx_sub = ctx_profile_toggle.clone();
                    toggle_action.on_toggle(move |active| {
                        let sub_cmd = if active { "up" } else { "down" };
                        let cmd = Cmd::new("nmcli")
                            .arg("connection")
                            .arg(sub_cmd)
                            .arg(&name_clone);
                        ctx_sub.daemon(cmd, KeepOnHide(false));
                    });

                    let status_str = if prof.active { "ACTIVE" } else { "IDLE" };
                    let item_row = nm_list_row(
                        "vpn",
                        &prof.name,
                        Some(status_str),
                        [toggle_action.widget().clone().upcast::<gtk4::Widget>()],
                    );
                    list_for_msg.append(&item_row);
                }
            }
        }
    });

    // 4. WARP Toggle Action Handler
    let ctx_warp = ctx.clone();
    let tx_warp_toggle = tx.clone();
    warp_toggle.on_toggle(move |active| {
        let action = if active { "connect" } else { "disconnect" };
        let cmd = Cmd::new("warp-cli").arg(action);
        let tx_inner = tx_warp_toggle.clone();
        ctx_warp.run(cmd, tx_inner, move |_| {
            VpnMsg::WarpStatus {
                connected: active,
                status_text: if active { "CONNECTED".to_string() } else { "DISCONNECTED".to_string() },
            }
        });
    });

    // 5. Periodic Polling Timer (Paused when tab hidden)
    let poll_tx = tx;
    let ctx_poll = ctx.clone();
    ctx.every(Duration::from_secs(5), move || {
        // Query WARP
        let warp_cmd = Cmd::new("warp-cli")
            .arg("status")
            .timeout(Duration::from_millis(1500));
        let tx_warp = poll_tx.clone();
        ctx_poll.run(warp_cmd, tx_warp, |res| {
            let connected = res.stdout.to_lowercase().contains("status update: connected");
            let text = if connected {
                "CONNECTED".to_string()
            } else if res.stdout.to_lowercase().contains("connecting") {
                "CONNECTING".to_string()
            } else {
                "DISCONNECTED".to_string()
            };
            VpnMsg::WarpStatus {
                connected,
                status_text: text,
            }
        });

        // Query NMCLI WireGuard & VPN profiles
        let nm_cmd = Cmd::new("nmcli")
            .args(["-t", "-f", "NAME,UUID,TYPE,DEVICE,STATE", "connection", "show"])
            .timeout(Duration::from_millis(1500));
        let tx_nm = poll_tx.clone();
        ctx_poll.run(nm_cmd, tx_nm, |res| {
            let mut list = Vec::new();
            if res.code == Some(0) {
                for line in res.stdout.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 {
                        let ctype = parts[2].to_lowercase();
                        if ctype == "wireguard" || ctype == "vpn" {
                            let is_active = parts.get(4).map(|s| s.to_lowercase() == "activated").unwrap_or(false);
                            list.push(VpnProfile {
                                name: parts[0].to_string(),
                                uuid: parts[1].to_string(),
                                conn_type: ctype,
                                active: is_active,
                            });
                        }
                    }
                }
            }
            VpnMsg::NmcliProfiles(list)
        });
    });

    let page = nm_page([
        warp_card.upcast::<gtk4::Widget>(),
        profile_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
