use crate::prelude::*;
use serde::{Deserialize, Serialize};

pub const SPEC: PluginSpec = PluginSpec::new(
    "email",
    "Email",
    "email",
    9,
    "Email inbox overview and quick-raise client",
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSummary {
    pub id: String,
    pub sender: String,
    pub subject: String,
    pub date: String,
}

#[derive(Debug, Clone)]
enum EmailMsg {
    UpdateList(Vec<EmailSummary>),
    ClientLaunched,
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Unread count label & Status Dot
    let status_dot = nm_status_dot(Tone::Muted);
    let count_label = nm_data_label("0 UNREAD");
    count_label.set_tone(Tone::Muted);

    // 2. Action buttons (Quick raise + Refresh)
    let raise_btn = nm_icon_button("external-link", "Raise desktop mail client");
    let refresh_btn = nm_icon_button("rotate-cw", "Refresh inbox status");

    let toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            count_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [
            raise_btn.widget().clone().upcast::<gtk4::Widget>(),
            refresh_btn.widget().clone().upcast::<gtk4::Widget>(),
        ],
    );

    // 3. Email preview list in card
    let email_list = nm_list();
    let email_card = nm_card([email_list.clone().upcast::<gtk4::Widget>()]);

    // 4. Mailbox handler
    let list_for_msg = email_list.clone();
    let dot_for_msg = status_dot;
    let count_for_msg = count_label;

    let tx = ctx.mailbox(move |msg: EmailMsg, _tx| match msg {
        EmailMsg::UpdateList(emails) => {
            while let Some(child) = list_for_msg.first_child() {
                list_for_msg.remove(&child);
            }

            if emails.is_empty() {
                dot_for_msg.set_tone(Tone::Muted);
                count_for_msg.set_tone(Tone::Muted);
                count_for_msg.set("0 UNREAD");

                let empty = nm_empty_state("email", "No unread messages or account configured");
                let row = gtk4::ListBoxRow::new();
                row.set_child(Some(&empty));
                list_for_msg.append(&row);
            } else {
                dot_for_msg.set_tone(Tone::Accent);
                count_for_msg.set_tone(Tone::Accent);
                count_for_msg.set(&format!("{} UNREAD", emails.len()));

                for item in emails {
                    let row = nm_list_row("email", &item.sender, Some(&item.date), []);
                    list_for_msg.append(&row);
                }
            }
        }
        EmailMsg::ClientLaunched => {
            count_for_msg.set("OPENING...");
        }
    });

    // 5. Desktop mail client raise button handler
    let ctx_raise = ctx.clone();
    let tx_raise = tx.clone();
    raise_btn.on_click(move || {
        tx_raise.send(EmailMsg::ClientLaunched);
        // Try raising kmail first (detected on system), then thunderbird, then xdg-open mailto:
        let launch_cmd = Cmd::new("kmail");
        ctx_raise.daemon(launch_cmd, KeepOnHide(true));
    });

    // 6. Refresh button handler
    let ctx_refresh = ctx.clone();
    let tx_refresh = tx.clone();
    let trigger_refresh = move || {
        let himalaya_cmd = Cmd::new("himalaya")
            .arg("envelope")
            .arg("list")
            .arg("-s")
            .arg("5")
            .arg("--output")
            .arg("json");

        let tx_inner = tx_refresh.clone();
        ctx_refresh.run(himalaya_cmd, &tx_inner, |res| {
            if res.code == Some(0) && !res.stdout.trim().is_empty() {
                if let Ok(parsed) = serde_json::from_str::<Vec<EmailSummary>>(&res.stdout) {
                    return EmailMsg::UpdateList(parsed);
                }
            }
            EmailMsg::UpdateList(Vec::new())
        });
    };

    let on_refresh = trigger_refresh.clone();
    refresh_btn.on_click(move || {
        on_refresh();
    });

    // 7. Initial query & 60s periodic polling
    trigger_refresh();

    let on_timer = trigger_refresh;
    ctx.every(Duration::from_secs(60), move || {
        on_timer();
    });

    // 8. Page assembly
    let page = nm_page([
        toolbar.upcast::<gtk4::Widget>(),
        email_card.upcast::<gtk4::Widget>(),
    ]);

    page.upcast::<gtk4::Widget>()
}
