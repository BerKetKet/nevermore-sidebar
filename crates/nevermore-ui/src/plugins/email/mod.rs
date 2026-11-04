use nevermore_ui::prelude::*;
use std::io::{Read, Write};
use std::net::TcpListener;

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

        let (tx, rx) = async_channel::bounded::<Result<(String, Option<String>), String>>(1);
        std::thread::spawn(move || {
            let res = authorize_gmail_loopback_worker(&client_id, &client_secret);
            let _ = tx.send_blocking(res);
        });

        let ctx_save = ctx_auth.clone();
        let status_lbl = status_clone.clone();
        glib::spawn_future_local(async move {
            if let Ok(res) = rx.recv().await {
                match res {
                    Ok((refresh_token, access_token)) => {
                        let _ = ctx_save.secret_set_blocking("gmail_refresh_token", &refresh_token);
                        if let Some(access) = access_token {
                            let _ = ctx_save.secret_set_blocking("gmail_access_token", &access);
                        }
                        status_lbl.set("AUTHORIZED");
                        status_lbl.set_tone(Tone::Success);
                    }
                    Err(err) => {
                        status_lbl.set(&format!("AUTH FAILED: {}", err));
                        status_lbl.set_tone(Tone::Danger);
                    }
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
    UpdateList(Vec<EmailSummary>, Option<String>),
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

    let render_emails = move |container: &gtk4::Box, emails: &[EmailSummary], tx_ref: &Tx<EmailMsg>| {
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
                let thread_url = format!("https://mail.google.com/mail/u/0/#inbox/{}", email.thread_id);
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

    let render_box = box_for_msg.clone();
    let tx = ctx.mailbox(move |msg: EmailMsg, tx_mailbox| match msg {
        EmailMsg::UpdateList(emails, maybe_new_tok) => {
            if let Some(tok) = maybe_new_tok {
                let _ = ctx_msg.secret_set_blocking("gmail_access_token", &tok);
            }
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
        EmailMsg::MarkRead(id) => {
            let client_id = ctx_msg
                .setting_str("client_id")
                .or_else(|| std::env::var("GMAIL_CLIENT_ID").ok())
                .unwrap_or_default();
            let client_secret = ctx_msg
                .secret_get_blocking("client_secret")
                .or_else(|| std::env::var("GMAIL_CLIENT_SECRET").ok())
                .unwrap_or_default();
            let refresh_token = ctx_msg
                .secret_get_blocking("gmail_refresh_token")
                .or_else(|| std::env::var("GMAIL_REFRESH_TOKEN").ok())
                .unwrap_or_default();
            let access_token = ctx_msg.secret_get_blocking("gmail_access_token");

            let tx_refresh = tx_mailbox.clone();
            std::thread::spawn(move || {
                let _ = mark_as_read_worker(
                    &client_id,
                    &client_secret,
                    &refresh_token,
                    access_token,
                    &id,
                );
                tx_refresh.send(EmailMsg::RefreshTriggered);
            });
        }
        EmailMsg::RefreshTriggered => {
            let client_id = ctx_msg
                .setting_str("client_id")
                .or_else(|| std::env::var("GMAIL_CLIENT_ID").ok())
                .unwrap_or_default();
            let client_secret = ctx_msg
                .secret_get_blocking("client_secret")
                .or_else(|| std::env::var("GMAIL_CLIENT_SECRET").ok())
                .unwrap_or_default();
            let refresh_token = ctx_msg
                .secret_get_blocking("gmail_refresh_token")
                .or_else(|| std::env::var("GMAIL_REFRESH_TOKEN").ok())
                .unwrap_or_default();
            let access_token = ctx_msg.secret_get_blocking("gmail_access_token");

            if refresh_token.is_empty() {
                tx_mailbox.send(EmailMsg::ErrorOccurred("NOT CONFIGURED".to_string()));
                return;
            }

            let tx_out = tx_mailbox.clone();
            std::thread::spawn(move || {
                match fetch_unread_messages_worker(
                    &client_id,
                    &client_secret,
                    &refresh_token,
                    access_token,
                ) {
                    Ok((list, new_tok)) => tx_out.send(EmailMsg::UpdateList(list, new_tok)),
                    Err(err) => tx_out.send(EmailMsg::ErrorOccurred(err)),
                }
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

    let page = nm_page([
        toolbar_card.upcast::<gtk4::Widget>(),
        email_container.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}

fn authorize_gmail_loopback_worker(
    client_id: &str,
    client_secret: &str,
) -> Result<(String, Option<String>), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Failed to bind loopback listener: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Get port failed: {}", e))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{}", port);

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fgmail.modify&access_type=offline&prompt=consent",
        client_id,
        url_encode(&redirect_uri)
    );

    let _ = Cmd::new("xdg-open").arg(&auth_url).spawn_detached();

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

    let post_body = format!(
        "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
        url_encode(&code),
        url_encode(client_id),
        url_encode(client_secret),
        url_encode(&redirect_uri)
    );

    let curl_cfg = format!(
        "url = \"https://oauth2.googleapis.com/token\"\nheader = \"Content-Type: application/x-www-form-urlencoded\"\ndata = \"{}\"\n",
        post_body.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let cmd = Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15));

    let res = cmd.execute_blocking();
    if res.code != Some(0) {
        return Err(format!("Token exchange failed: {}", res.stderr));
    }

    let token_json: serde_json::Value = serde_json::from_str(&res.stdout)
        .map_err(|e| format!("Invalid token response: {}", e))?;

    let refresh_token = token_json["refresh_token"]
        .as_str()
        .ok_or_else(|| "No refresh token received".to_string())?
        .to_string();

    let access_token = token_json["access_token"].as_str().map(|s| s.to_string());

    Ok((refresh_token, access_token))
}

fn get_valid_access_token_worker(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    access_token: Option<String>,
) -> Result<(String, Option<String>), String> {
    if let Some(tok) = access_token {
        let trimmed = tok.trim();
        if !trimmed.is_empty() {
            return Ok((trimmed.to_string(), None));
        }
    }

    if refresh_token.trim().is_empty() {
        return Err("Gmail not authorized".to_string());
    }

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

    let cmd = Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15));

    let res = cmd.execute_blocking();
    if res.code != Some(0) {
        return Err(format!("Token refresh failed: {}", res.stderr));
    }

    let token_json: serde_json::Value = serde_json::from_str(&res.stdout)
        .map_err(|e| format!("Invalid refresh response: {}", e))?;

    let new_access = token_json["access_token"]
        .as_str()
        .ok_or_else(|| "No access_token in refresh response".to_string())?
        .to_string();

    Ok((new_access.clone(), Some(new_access)))
}

fn fetch_unread_messages_worker(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    access_token: Option<String>,
) -> Result<(Vec<EmailSummary>, Option<String>), String> {
    let (token, new_token) =
        get_valid_access_token_worker(client_id, client_secret, refresh_token, access_token)?;

    let list_url = "https://gmail.googleapis.com/gmail/v1/users/me/messages?q=is:unread+in:inbox&maxResults=15";
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
        list_url, token
    );

    let cmd = Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15));

    let res = cmd.execute_blocking();
    if res.code != Some(0) {
        return Err(format!("Fetch messages failed: {}", res.stderr));
    }

    let json: serde_json::Value = serde_json::from_str(&res.stdout)
        .map_err(|e| format!("Parse messages json failed: {}", e))?;

    let messages = match json["messages"].as_array() {
        Some(arr) => arr,
        None => return Ok((Vec::new(), new_token)),
    };

    let mut summaries = Vec::new();
    for msg in messages {
        let msg_id = match msg["id"].as_str() {
            Some(id) => id,
            None => continue,
        };
        let thread_id = msg["threadId"].as_str().unwrap_or(msg_id).to_string();

        let detail_url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=metadata&metadataHeaders=From&metadataHeaders=Subject",
            msg_id
        );
        let detail_cfg = format!(
            "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
            detail_url, token
        );

        let d_cmd = Cmd::new("curl")
            .args(["-fsS", "--max-time", "10", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
            .stdin_bytes(detail_cfg.into_bytes())
            .timeout(Duration::from_secs(10));

        let d_res = d_cmd.execute_blocking();
        if d_res.code == Some(0) {
            if let Ok(d_json) = serde_json::from_str::<serde_json::Value>(&d_res.stdout) {
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

                summaries.push(EmailSummary {
                    id: msg_id.to_string(),
                    thread_id,
                    from: sender,
                    subject,
                    snippet,
                });
            }
        }
    }

    Ok((summaries, new_token))
}

fn mark_as_read_worker(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    access_token: Option<String>,
    msg_id: &str,
) -> Result<(), String> {
    let (token, _) =
        get_valid_access_token_worker(client_id, client_secret, refresh_token, access_token)?;
    let modify_url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify",
        msg_id
    );
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\nheader = \"Content-Type: application/json\"\ndata = \"{{\\\"removeLabelIds\\\":[\\\"UNREAD\\\"]}}\"\n",
        modify_url, token
    );

    let cmd = Cmd::new("curl")
        .args(["-fsS", "--max-time", "10", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(10));

    let res = cmd.execute_blocking();
    if res.code == Some(0) {
        Ok(())
    } else {
        Err(res.stderr)
    }
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
