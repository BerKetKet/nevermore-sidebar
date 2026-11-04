use nevermore_ui::prelude::*;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::rc::Rc;

pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new("client_id", "Google OAuth Client ID", SettingKind::Text),
    SettingSpec::new(
        "client_secret",
        "Google OAuth Client Secret",
        SettingKind::Secret,
    ),
    SettingSpec::new("poll_interval_sec", "Poll Interval (s)", SettingKind::Int),
];

pub fn settings_custom(ctx: &PluginContext) -> gtk4::Widget {
    let auth_btn = nm_icon_button("key", "Authorize Gmail Account");
    let is_auth = ctx.secret_get_blocking("gmail_refresh_token").is_some();
    let status_label = nm_data_label(if is_auth {
        "AUTHORIZED"
    } else {
        "NOT AUTHORIZED"
    });
    status_label.set_tone(if is_auth {
        Tone::Success
    } else {
        Tone::Warning
    });

    let ctx_auth = ctx.clone();
    let status_clone = status_label.clone();
    auth_btn.on_click(move || {
        let client_id = ctx_auth
            .setting_str("client_id")
            .or_else(|| std::env::var("GMAIL_CLIENT_ID").ok());
        let client_secret = ctx_auth
            .secret_get_blocking("client_secret")
            .or_else(|| std::env::var("GMAIL_CLIENT_SECRET").ok());

        let (client_id, client_secret) = match (client_id, client_secret) {
            (Some(id), Some(sec)) if !id.trim().is_empty() && !sec.trim().is_empty() => (id, sec),
            _ => {
                status_clone.set("CONFIG REQUIRED");
                status_clone.set_tone(Tone::Warning);
                return;
            }
        };

        status_clone.set("AUTHORIZING...");
        status_clone.set_tone(Tone::Accent);

        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(e) => {
                status_clone.set(&format!("BIND FAILED: {}", e));
                status_clone.set_tone(Tone::Danger);
                return;
            }
        };

        let port = match listener.local_addr() {
            Ok(a) => a.port(),
            Err(e) => {
                status_clone.set(&format!("PORT FAILED: {}", e));
                status_clone.set_tone(Tone::Danger);
                return;
            }
        };

        let redirect_uri = format!("http://127.0.0.1:{}", port);
        let auth_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fgmail.modify&access_type=offline&prompt=consent",
            client_id,
            url_encode(&redirect_uri)
        );

        let _ = Cmd::new("xdg-open").arg(&auth_url).spawn_detached();

        let (code_tx, code_rx) = async_channel::bounded::<Result<String, String>>(1);
        std::thread::spawn(move || {
            let res = wait_for_redirect_code(listener);
            let _ = code_tx.send_blocking(res);
        });

        let ctx_save = ctx_auth.clone();
        let status_lbl = status_clone.clone();
        glib::spawn_future_local(async move {
            match code_rx.recv().await {
                Ok(Ok(code)) => {
                    let token_cmd =
                        build_exchange_cmd(&code, &client_id, &client_secret, &redirect_uri);
                    let (tok_tx, tok_rx) = async_channel::bounded::<CmdResult>(1);
                    ctx_save.run(token_cmd, &Tx::new(tok_tx), |res| res);
                    if let Ok(cmd_res) = tok_rx.recv().await {
                        if cmd_res.code == Some(0) {
                            if let Ok(token_json) =
                                serde_json::from_str::<serde_json::Value>(&cmd_res.stdout)
                            {
                                if let Some(refresh_token) = token_json["refresh_token"].as_str() {
                                    let _ = ctx_save.secret_set_blocking(
                                        "gmail_refresh_token",
                                        refresh_token,
                                    );
                                    if let Some(access_token) = token_json["access_token"].as_str()
                                    {
                                        let _ = ctx_save.secret_set_blocking(
                                            "gmail_access_token",
                                            access_token,
                                        );
                                    }
                                    status_lbl.set("AUTHORIZED");
                                    status_lbl.set_tone(Tone::Success);
                                    return;
                                }
                            }
                        }
                        status_lbl.set("EXCHANGE FAILED");
                        status_lbl.set_tone(Tone::Danger);
                    }
                }
                Ok(Err(err)) => {
                    status_lbl.set(&format!("AUTH FAILED: {}", err));
                    status_lbl.set_tone(Tone::Danger);
                }
                Err(_) => {
                    status_lbl.set("CANCELLED");
                    status_lbl.set_tone(Tone::Warning);
                }
            }
        });
    });

    let card = nm_card([
        nm_section_header("GMAIL OAUTH2 AUTHORIZATION").upcast::<gtk4::Widget>(),
        nm_field_label("Click to sign in via Google OAuth2").upcast::<gtk4::Widget>(),
        auth_btn.widget().clone().upcast::<gtk4::Widget>(),
        status_label.widget().clone().upcast::<gtk4::Widget>(),
    ]);
    card.upcast::<gtk4::Widget>()
}

pub const SPEC: PluginSpec = PluginSpec::with_settings(
    "email",
    "Gmail Inbox",
    "mail",
    8,
    "Official Gmail REST API v1 unread message inbox",
    SETTINGS,
    Some(settings_custom),
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmailSummary {
    pub id: String,
    pub thread_id: String,
    pub from: String,
    pub subject: String,
    pub snippet: String,
}

#[derive(Clone, Debug)]
pub enum EmailMsg {
    UpdateList(Vec<EmailSummary>),
    ListFetched {
        messages: Vec<(String, String)>,
    },
    DetailFetched {
        summary: EmailSummary,
        expected: usize,
    },
    TokenRefreshed {
        token: String,
    },
    MarkRead(String),
    RefreshTriggered,
    ErrorOccurred(String),
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let email_container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    email_container.set_hexpand(true);

    let status_dot = nm_status_dot(Tone::Secondary);
    let count_label = nm_data_label("0 UNREAD");
    count_label.set_tone(Tone::Secondary);

    let refresh_btn = nm_icon_button("refresh-cw", "Refresh Gmail Inbox");

    let toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            count_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [refresh_btn.widget().clone().upcast::<gtk4::Widget>()],
    );
    let toolbar_card = nm_card([toolbar.upcast::<gtk4::Widget>()]);

    let box_for_msg = email_container.clone();
    let dot_for_msg = status_dot;
    let count_for_msg = count_label;
    let ctx_msg = ctx.clone();

    let render_emails =
        move |container: &gtk4::Box, emails: &[EmailSummary], tx_ref: &Tx<EmailMsg>| {
            while let Some(child) = container.first_child() {
                container.remove(&child);
            }

            if emails.is_empty() {
                let empty = nm_empty_state("mail", "No unread messages");
                container.append(&empty);
            } else {
                for email in emails {
                    let card_items = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

                    let header_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                    let mail_icon = nm_icon("mail", Tone::Secondary, 16);
                    header_row.append(mail_icon.widget());

                    let sender_label = nm_data_label(&email.from);
                    sender_label.set_tone(Tone::Primary);
                    sender_label.set_hexpand(true);
                    header_row.append(sender_label.widget());

                    let open_btn = nm_icon_button("external-link", "Open Thread in Browser");
                    let thread_url =
                        format!("https://mail.google.com/mail/u/0/#inbox/{}", email.thread_id);
                    open_btn.on_click(move || {
                        let _ = Cmd::new("xdg-open").arg(&thread_url).spawn_detached();
                    });
                    header_row.append(open_btn.widget());

                    let mark_btn = nm_icon_button("check", "Mark as Read");
                    let id_for_mark = email.id.clone();
                    let tx_mark = tx_ref.clone();
                    mark_btn.on_click(move || {
                        tx_mark.send(EmailMsg::MarkRead(id_for_mark.clone()));
                    });
                    header_row.append(mark_btn.widget());

                    card_items.append(&header_row);

                    let subject_label = nm_data_label(&email.subject);
                    subject_label.set_tone(Tone::Secondary);
                    card_items.append(subject_label.widget());

                    if !email.snippet.is_empty() {
                        let snippet_prose = nm_prose_label(&email.snippet);
                        card_items.append(&snippet_prose);
                    }

                    let card = nm_card([card_items.upcast::<gtk4::Widget>()]);
                    container.append(&card);
                }
            }
        };

    let pending_emails: Rc<RefCell<Vec<EmailSummary>>> = Rc::new(RefCell::new(Vec::new()));
    let render_box = box_for_msg.clone();
    let pending_for_msg = pending_emails;

    let tx = ctx.mailbox(move |msg: EmailMsg, tx_mailbox| match msg {
        EmailMsg::UpdateList(emails) => {
            let count = emails.len();
            count_for_msg.set(&format!("{} UNREAD", count));
            if count > 0 {
                dot_for_msg.set_tone(Tone::Accent);
                count_for_msg.set_tone(Tone::Accent);
            } else {
                dot_for_msg.set_tone(Tone::Secondary);
                count_for_msg.set_tone(Tone::Muted);
            }
            render_emails(&render_box, &emails, tx_mailbox);
        }
        EmailMsg::ListFetched { messages } => {
            if messages.is_empty() {
                tx_mailbox.send(EmailMsg::UpdateList(Vec::new()));
                return;
            }

            pending_for_msg.borrow_mut().clear();
            let total = messages.len();
            let token = ctx_msg
                .secret_get_blocking("gmail_access_token")
                .unwrap_or_default();

            for (msg_id, thread_id) in messages {
                let detail_cmd = build_detail_cmd(&token, &msg_id);
                let tx_inner = tx_mailbox.clone();
                let m_id = msg_id.clone();
                let t_id = thread_id.clone();

                ctx_msg.run(detail_cmd, &tx_inner, move |res| {
                    if res.code == Some(0) {
                        if let Ok(d_json) = serde_json::from_str::<serde_json::Value>(&res.stdout) {
                            let mut sender = "Unknown".to_string();
                            let mut subject = "(No Subject)".to_string();
                            let snippet = d_json["snippet"].as_str().unwrap_or("").to_string();

                            if let Some(headers) = d_json["payload"]["headers"].as_array() {
                                for h in headers {
                                    let name = h["name"].as_str().unwrap_or("");
                                    let value = h["value"].as_str().unwrap_or("");
                                    if name.eq_ignore_ascii_case("From") {
                                        sender = value.to_string();
                                    } else if name.eq_ignore_ascii_case("Subject") {
                                        subject = value.to_string();
                                    }
                                }
                            }

                            return EmailMsg::DetailFetched {
                                summary: EmailSummary {
                                    id: m_id,
                                    thread_id: t_id,
                                    from: sender,
                                    subject,
                                    snippet,
                                },
                                expected: total,
                            };
                        }
                    }

                    EmailMsg::DetailFetched {
                        summary: EmailSummary {
                            id: m_id,
                            thread_id: t_id,
                            from: "Unknown".to_string(),
                            subject: "(No Subject)".to_string(),
                            snippet: String::new(),
                        },
                        expected: total,
                    }
                });
            }
        }
        EmailMsg::DetailFetched { summary, expected } => {
            let mut list = pending_for_msg.borrow_mut();
            list.push(summary);
            if list.len() >= expected {
                let emails = list.clone();
                drop(list);
                tx_mailbox.send(EmailMsg::UpdateList(emails));
            }
        }
        EmailMsg::TokenRefreshed { token } => {
            let _ = ctx_msg.secret_set_blocking("gmail_access_token", &token);
            let list_cmd = build_list_cmd(&token);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(list_cmd, &tx_inner, move |res| {
                if res.code != Some(0) {
                    return EmailMsg::ErrorOccurred("SYNC FAILED".to_string());
                }
                let mut msgs = Vec::new();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&res.stdout) {
                    if let Some(arr) = json["messages"].as_array() {
                        for m in arr {
                            if let Some(id) = m["id"].as_str() {
                                let tid = m["threadId"].as_str().unwrap_or(id);
                                msgs.push((id.to_string(), tid.to_string()));
                            }
                        }
                    }
                }
                EmailMsg::ListFetched { messages: msgs }
            });
        }
        EmailMsg::MarkRead(id) => {
            let token = ctx_msg
                .secret_get_blocking("gmail_access_token")
                .unwrap_or_default();
            let modify_cmd = build_modify_cmd(&token, &id);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(modify_cmd, &tx_inner, move |_| {
                EmailMsg::RefreshTriggered
            });
        }
        EmailMsg::RefreshTriggered => {
            let refresh_token = ctx_msg
                .secret_get_blocking("gmail_refresh_token")
                .or_else(|| std::env::var("GMAIL_REFRESH_TOKEN").ok())
                .unwrap_or_default();

            if refresh_token.is_empty() {
                tx_mailbox.send(EmailMsg::ErrorOccurred("NOT CONFIGURED".to_string()));
                return;
            }

            let client_id = ctx_msg
                .setting_str("client_id")
                .or_else(|| std::env::var("GMAIL_CLIENT_ID").ok())
                .unwrap_or_default();
            let client_secret = ctx_msg
                .secret_get_blocking("client_secret")
                .or_else(|| std::env::var("GMAIL_CLIENT_SECRET").ok())
                .unwrap_or_default();

            let refresh_cmd = build_refresh_cmd(&client_id, &client_secret, &refresh_token);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(refresh_cmd, &tx_inner, move |res| {
                if res.code != Some(0) {
                    return EmailMsg::ErrorOccurred("AUTH ERROR".to_string());
                }
                if let Ok(token_json) = serde_json::from_str::<serde_json::Value>(&res.stdout) {
                    if let Some(new_access) = token_json["access_token"].as_str() {
                        return EmailMsg::TokenRefreshed {
                            token: new_access.to_string(),
                        };
                    }
                }
                EmailMsg::ErrorOccurred("TOKEN ERROR".to_string())
            });
        }
        EmailMsg::ErrorOccurred(_err) => {
            dot_for_msg.set_tone(Tone::Warning);
            count_for_msg.set("SYNC ERROR");
            count_for_msg.set_tone(Tone::Warning);
        }
    });

    let tx_refresh = tx.clone();
    refresh_btn.on_click(move || {
        tx_refresh.send(EmailMsg::RefreshTriggered);
    });

    let poll_interval = ctx
        .setting_int("poll_interval_sec")
        .unwrap_or(120)
        .max(30) as u64;

    let tx_poll = tx.clone();
    ctx.every(Duration::from_secs(poll_interval), move || {
        tx_poll.send(EmailMsg::RefreshTriggered);
    });

    // Initial check on startup/build
    tx.send(EmailMsg::RefreshTriggered);

    let page = nm_page([
        toolbar_card.upcast::<gtk4::Widget>(),
        email_container.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}

fn wait_for_redirect_code(listener: TcpListener) -> Result<String, String> {
    listener.set_nonblocking(false).ok();
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("Accept failed: {}", e))?;

    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).map_err(|e| format!("Read failed: {}", e))?;
    let req_str = String::from_utf8_lossy(&buf[..n]);

    let code = extract_param(&req_str, "code=")
        .ok_or_else(|| "Authorization code not found in redirect".to_string())?;

    let html_resp = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<html><body style='font-family:sans-serif;padding:40px;background:#18181b;color:#f4f4f5;'><h2>Nevermore: Gmail Authorized</h2><p>You can close this browser tab.</p></body></html>";
    let _ = stream.write_all(html_resp.as_bytes());

    Ok(code)
}

fn build_exchange_cmd(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Cmd {
    let post_body = format!(
        "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
        url_encode(code),
        url_encode(client_id),
        url_encode(client_secret),
        url_encode(redirect_uri)
    );

    let curl_cfg = format!(
        "url = \"https://oauth2.googleapis.com/token\"\nheader = \"Content-Type: application/x-www-form-urlencoded\"\ndata = \"{}\"\n",
        post_body.replace('\\', "\\\\").replace('"', "\\\"")
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15))
}

fn build_refresh_cmd(client_id: &str, client_secret: &str, refresh_token: &str) -> Cmd {
    let post_body = format!(
        "client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token",
        url_encode(client_id),
        url_encode(client_secret),
        url_encode(refresh_token)
    );

    let curl_cfg = format!(
        "url = \"https://oauth2.googleapis.com/token\"\nheader = \"Content-Type: application/x-www-form-urlencoded\"\ndata = \"{}\"\n",
        post_body.replace('\\', "\\\\").replace('"', "\\\"")
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15))
}

fn build_list_cmd(token: &str) -> Cmd {
    let list_url = "https://gmail.googleapis.com/gmail/v1/users/me/messages?q=is:unread+in:inbox&maxResults=15";
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
        list_url, token
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15))
}

fn build_detail_cmd(token: &str, msg_id: &str) -> Cmd {
    let detail_url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata&metadataHeaders=From&metadataHeaders=Subject",
        msg_id
    );
    let detail_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
        detail_url, token
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "10", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(detail_cfg.into_bytes())
        .timeout(Duration::from_secs(10))
}

fn build_modify_cmd(token: &str, msg_id: &str) -> Cmd {
    let modify_url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify",
        msg_id
    );
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\nheader = \"Content-Type: application/json\"\ndata = \"{{\\\"removeLabelIds\\\":[\\\"UNREAD\\\"]}}\"\n",
        modify_url, token
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "10", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(10))
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn extract_param(s: &str, prefix: &str) -> Option<String> {
    let start = s.find(prefix)? + prefix.len();
    let rem = &s[start..];
    let end = rem.find('&').or_else(|| rem.find(' ')).unwrap_or(rem.len());
    Some(rem[..end].to_string())
}
