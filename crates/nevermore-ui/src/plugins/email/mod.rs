use nevermore_ui::prelude::*;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::rc::Rc;

pub mod mime;

pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new(
        "client_id",
        "Google OAuth Client ID",
        SettingKind::Text { default: "" },
    ),
    SettingSpec::new(
        "client_secret",
        "Google OAuth Client Secret",
        SettingKind::Secret {
            key_name: "client_secret",
        },
    ),
    SettingSpec::new(
        "poll_interval_sec",
        "Poll Interval (s)",
        SettingKind::Int {
            default: 120,
            min: 10,
            max: 86400,
        },
    ),
];

pub fn settings_custom(ctx: &PluginContext) -> gtk4::Widget {
    let auth_btn = nm_icon_button("key", "Authorize Gmail Account");
    let status_label = nm_data_label("CHECKING...");
    status_label.set_tone(Tone::Muted);

    let (tx_status, rx_status) = async_channel::unbounded::<(String, Tone)>();
    let status_for_rx = status_label.clone();
    glib::spawn_future_local(async move {
        while let Ok((msg, tone)) = rx_status.recv().await {
            status_for_rx.set(&msg);
            status_for_rx.set_tone(tone);
        }
    });

    let pid_check = ctx.plugin_id.clone();
    let tx_check = tx_status.clone();
    std::thread::spawn(move || {
        let is_auth = nevermore_ui::plugin_api::secret_lookup(&pid_check, "gmail_refresh_token")
            .is_some()
            || std::env::var("GMAIL_REFRESH_TOKEN").is_ok();
        let _ = tx_check.send_blocking(if is_auth {
            ("AUTHORIZED".to_string(), Tone::Success)
        } else {
            ("NOT AUTHORIZED".to_string(), Tone::Warning)
        });
    });

    let pid_auth = ctx.plugin_id.clone();
    let tx_auth = tx_status.clone();
    let client_id_opt = ctx.setting_str("client_id");
    auth_btn.on_click(move || {
        let pid_bg = pid_auth.clone();
        let tx_bg = tx_auth.clone();
        let client_id_opt = client_id_opt.clone();

        let _ = tx_bg.send_blocking(("CONFIGURING...".to_string(), Tone::Accent));

        std::thread::spawn(move || {
            let client_id = client_id_opt
                .or_else(|| std::env::var("GMAIL_CLIENT_ID").ok());
            let client_secret = nevermore_ui::plugin_api::secret_lookup(&pid_bg, "client_secret")
                .or_else(|| std::env::var("GMAIL_CLIENT_SECRET").ok());

            let (client_id, client_secret) = match (client_id, client_secret) {
                (Some(id), Some(sec)) if !id.trim().is_empty() && !sec.trim().is_empty() => {
                    (id, sec)
                }
                _ => {
                    let _ = tx_bg.send_blocking(("CONFIG REQUIRED".to_string(), Tone::Warning));
                    return;
                }
            };

            let listener = match TcpListener::bind("127.0.0.1:0") {
                Ok(l) => l,
                Err(e) => {
                    let _ = tx_bg.send_blocking((format!("BIND FAILED: {}", e), Tone::Danger));
                    return;
                }
            };

            let port = match listener.local_addr() {
                Ok(a) => a.port(),
                Err(e) => {
                    let _ = tx_bg.send_blocking((format!("PORT FAILED: {}", e), Tone::Danger));
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

            let _ = tx_bg.send_blocking(("AUTHORIZING...".to_string(), Tone::Accent));

            let res = wait_for_redirect_code(listener);
            match res {
                Ok(code) => {
                    let token_cmd =
                        build_exchange_cmd(&code, &client_id, &client_secret, &redirect_uri);
                    let cmd_res = token_cmd.run_in_worker_thread();
                    if cmd_res.code == Some(0) {
                        if let Ok(token_json) =
                            serde_json::from_str::<serde_json::Value>(&cmd_res.stdout)
                        {
                            if let Some(refresh_token) = token_json["refresh_token"].as_str() {
                                let ref_tok = refresh_token.to_string();
                                let acc_tok = token_json["access_token"].as_str().map(|s| s.to_string());
                                let _ = nevermore_ui::plugin_api::secret_store(&pid_bg, "gmail_refresh_token", &ref_tok);
                                if let Some(acc) = acc_tok {
                                    let _ = nevermore_ui::plugin_api::secret_store(&pid_bg, "gmail_access_token", &acc);
                                }
                                let _ = tx_bg.send_blocking(("AUTHORIZED".to_string(), Tone::Success));
                                return;
                            }
                        }
                    }
                    let _ = tx_bg.send_blocking(("EXCHANGE FAILED".to_string(), Tone::Danger));
                }
                Err(err) => {
                    let _ = tx_bg.send_blocking((format!("AUTH FAILED: {}", err), Tone::Danger));
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
    CredentialsLoaded {
        refresh_token: String,
        client_id: String,
        client_secret: String,
    },
    OpenEmailInBlade {
        id: String,
        focus_reply: bool,
    },
    FullMessageLoaded {
        msg: mime::ParsedMessage,
        focus_reply: bool,
    },
    SendReply {
        thread_id: String,
        raw_b64: String,
    },
    MarkRead(String),
    MarkSpam(String),
    Trash(String),
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

                    let action_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);

                    let read_btn = nm_icon_button("mail", "Read Email in Companion Blade");
                    let id_read = email.id.clone();
                    let tx_read = tx_ref.clone();
                    read_btn.on_click(move || {
                        tx_read.send(EmailMsg::OpenEmailInBlade {
                            id: id_read.clone(),
                            focus_reply: false,
                        });
                    });
                    action_row.append(read_btn.widget());

                    let reply_btn = nm_icon_button("corner-up-left", "Reply in Companion Blade");
                    let id_reply = email.id.clone();
                    let tx_reply = tx_ref.clone();
                    reply_btn.on_click(move || {
                        tx_reply.send(EmailMsg::OpenEmailInBlade {
                            id: id_reply.clone(),
                            focus_reply: true,
                        });
                    });
                    action_row.append(reply_btn.widget());

                    let mark_btn = nm_icon_button("check", "Mark as Read");
                    let id_mark = email.id.clone();
                    let tx_mark = tx_ref.clone();
                    mark_btn.on_click(move || {
                        tx_mark.send(EmailMsg::MarkRead(id_mark.clone()));
                    });
                    action_row.append(mark_btn.widget());

                    let spam_btn = nm_icon_button("shield-off", "Mark as Spam");
                    let id_spam = email.id.clone();
                    let tx_spam = tx_ref.clone();
                    spam_btn.on_click(move || {
                        tx_spam.send(EmailMsg::MarkSpam(id_spam.clone()));
                    });
                    action_row.append(spam_btn.widget());

                    let trash_btn = nm_icon_button("trash-2", "Delete / Trash");
                    let id_trash = email.id.clone();
                    let tx_trash = tx_ref.clone();
                    trash_btn.on_click(move || {
                        tx_trash.send(EmailMsg::Trash(id_trash.clone()));
                    });
                    action_row.append(trash_btn.widget());

                    let open_btn = nm_icon_button("external-link", "Open Thread in Browser");
                    let thread_url =
                        format!("https://mail.google.com/mail/u/0/#inbox/{}", email.thread_id);
                    open_btn.on_click(move || {
                        let _ = Cmd::new("xdg-open").arg(&thread_url).spawn_detached();
                    });
                    action_row.append(open_btn.widget());

                    header_row.append(&action_row);
                    card_items.append(&header_row);

                    let subject_label = nm_data_label(&email.subject);
                    subject_label.set_tone(Tone::Secondary);
                    card_items.append(subject_label.widget());

                    if !email.snippet.is_empty() {
                        let snippet_prose = nm_prose_label(&email.snippet);
                        card_items.append(&snippet_prose);
                    }

                    let card = nm_card([card_items.upcast::<gtk4::Widget>()]);
                    let click_gesture = gtk4::GestureClick::new();
                    let id_click = email.id.clone();
                    let tx_click = tx_ref.clone();
                    click_gesture.connect_released(move |_, _, _, _| {
                        tx_click.send(EmailMsg::OpenEmailInBlade {
                            id: id_click.clone(),
                            focus_reply: false,
                        });
                    });
                    card.add_controller(click_gesture);

                    container.append(&card);
                }
            }
        };

    let pending_emails: Rc<RefCell<Vec<EmailSummary>>> = Rc::new(RefCell::new(Vec::new()));
    let render_box = box_for_msg.clone();
    let pending_for_msg = pending_emails;
    let cached_token: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let cached_token_for_msg = cached_token.clone();

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
            let token = cached_token_for_msg.borrow().clone().unwrap_or_default();

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
        EmailMsg::CredentialsLoaded {
            refresh_token,
            client_id,
            client_secret,
        } => {
            let refresh_cmd = build_refresh_cmd(&client_id, &client_secret, &refresh_token);
            let tx_tok = tx_mailbox.clone();
            ctx_msg.run(refresh_cmd, &tx_tok, move |res| {
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
        EmailMsg::TokenRefreshed { token } => {
            *cached_token_for_msg.borrow_mut() = Some(token.clone());
            let pid = ctx_msg.plugin_id.clone();
            let tok_to_save = token.clone();
            std::thread::spawn(move || {
                let _ = nevermore_ui::plugin_api::secret_store(&pid, "gmail_access_token", &tok_to_save);
            });
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
        EmailMsg::OpenEmailInBlade { id, focus_reply } => {
            let loading = nm_empty_state_with_tone("mail", Tone::Accent, "Loading email...");
            ctx_msg.open_companion_blade(&loading);

            let token = cached_token_for_msg.borrow().clone().unwrap_or_default();
            let cmd = build_full_message_cmd(&token, &id);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(cmd, &tx_inner, move |res| {
                if res.code == Some(0) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&res.stdout) {
                        let parsed = mime::parse_full_message(&json);
                        return EmailMsg::FullMessageLoaded {
                            msg: parsed,
                            focus_reply,
                        };
                    }
                }
                EmailMsg::ErrorOccurred("FETCH EMAIL FAILED".to_string())
            });
        }
        EmailMsg::FullMessageLoaded { msg, focus_reply } => {
            let reader = build_blade_reader(&msg, tx_mailbox, &ctx_msg, focus_reply);
            ctx_msg.open_companion_blade(&reader);
        }
        EmailMsg::SendReply { thread_id, raw_b64 } => {
            let token = cached_token_for_msg.borrow().clone().unwrap_or_default();
            let cmd = build_send_cmd(&token, &raw_b64, &thread_id);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(cmd, &tx_inner, move |res| {
                if res.code == Some(0) {
                    EmailMsg::RefreshTriggered
                } else {
                    EmailMsg::ErrorOccurred("SEND REPLY FAILED".to_string())
                }
            });
        }
        EmailMsg::MarkSpam(id) => {
            let token = cached_token_for_msg.borrow().clone().unwrap_or_default();
            let cmd = build_modify_cmd(&token, &id, &["SPAM"], &["INBOX", "UNREAD"]);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(cmd, &tx_inner, move |_| EmailMsg::RefreshTriggered);
        }
        EmailMsg::Trash(id) => {
            let token = cached_token_for_msg.borrow().clone().unwrap_or_default();
            let cmd = build_trash_cmd(&token, &id);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(cmd, &tx_inner, move |_| EmailMsg::RefreshTriggered);
        }
        EmailMsg::MarkRead(id) => {
            let token = cached_token_for_msg.borrow().clone().unwrap_or_default();
            let modify_cmd = build_modify_cmd(&token, &id, &[], &["UNREAD"]);
            let tx_inner = tx_mailbox.clone();
            ctx_msg.run(modify_cmd, &tx_inner, move |_| EmailMsg::RefreshTriggered);
        }
        EmailMsg::RefreshTriggered => {
            let pid = ctx_msg.plugin_id.clone();
            let client_id_opt = ctx_msg.setting_str("client_id");
            let tx_inner = tx_mailbox.clone();
            std::thread::spawn(move || {
                let refresh_token = nevermore_ui::plugin_api::secret_lookup(&pid, "gmail_refresh_token")
                    .or_else(|| std::env::var("GMAIL_REFRESH_TOKEN").ok())
                    .unwrap_or_default();

                if refresh_token.is_empty() {
                    tx_inner.send(EmailMsg::ErrorOccurred("NOT CONFIGURED".to_string()));
                    return;
                }

                let client_id = client_id_opt
                    .or_else(|| std::env::var("GMAIL_CLIENT_ID").ok())
                    .unwrap_or_default();
                let client_secret = nevermore_ui::plugin_api::secret_lookup(&pid, "client_secret")
                    .or_else(|| std::env::var("GMAIL_CLIENT_SECRET").ok())
                    .unwrap_or_default();

                tx_inner.send(EmailMsg::CredentialsLoaded {
                    refresh_token,
                    client_id,
                    client_secret,
                });
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

fn build_blade_reader(
    msg: &mime::ParsedMessage,
    tx: &Tx<EmailMsg>,
    ctx: &PluginContext,
    focus_reply: bool,
) -> gtk4::Widget {
    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    main_box.set_hexpand(true);
    main_box.set_vexpand(true);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);
    main_box.set_margin_top(12);
    main_box.set_margin_bottom(12);

    // 1. Top Toolbar & Header Card
    let subject_header = nm_section_header(&msg.subject);
    let from_label = nm_field_label(&format!("From: {}", msg.from));
    let date_label = nm_data_label(&msg.date);
    date_label.set_tone(Tone::Muted);

    let actions_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

    let reply_btn = nm_icon_button("corner-up-left", "Reply to Email");
    let mark_btn = nm_icon_button("check", "Mark as Read");
    let spam_btn = nm_icon_button("shield-off", "Mark as Spam");
    let trash_btn = nm_icon_button("trash-2", "Delete / Trash");
    let open_btn = nm_icon_button("external-link", "Open in Browser");
    let close_btn = nm_icon_button("x", "Close Blade");

    let thread_url = format!("https://mail.google.com/mail/u/0/#inbox/{}", msg.thread_id);
    open_btn.on_click(move || {
        let _ = Cmd::new("xdg-open").arg(&thread_url).spawn_detached();
    });

    let msg_id = msg.id.clone();
    let tx_mark = tx.clone();
    let ctx_mark = ctx.clone();
    mark_btn.on_click(move || {
        tx_mark.send(EmailMsg::MarkRead(msg_id.clone()));
        ctx_mark.close_companion_blade();
    });

    let msg_id_spam = msg.id.clone();
    let tx_spam = tx.clone();
    let ctx_spam = ctx.clone();
    spam_btn.on_click(move || {
        tx_spam.send(EmailMsg::MarkSpam(msg_id_spam.clone()));
        ctx_spam.close_companion_blade();
    });

    let msg_id_trash = msg.id.clone();
    let tx_trash = tx.clone();
    let ctx_trash = ctx.clone();
    trash_btn.on_click(move || {
        tx_trash.send(EmailMsg::Trash(msg_id_trash.clone()));
        ctx_trash.close_companion_blade();
    });

    let ctx_close = ctx.clone();
    close_btn.on_click(move || {
        ctx_close.close_companion_blade();
    });

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    actions_box.append(reply_btn.widget());
    actions_box.append(mark_btn.widget());
    actions_box.append(spam_btn.widget());
    actions_box.append(trash_btn.widget());
    actions_box.append(open_btn.widget());
    actions_box.append(&spacer);
    actions_box.append(close_btn.widget());

    let header_card = nm_card([
        subject_header.upcast::<gtk4::Widget>(),
        from_label.upcast::<gtk4::Widget>(),
        date_label.widget().clone().upcast::<gtk4::Widget>(),
        actions_box.upcast::<gtk4::Widget>(),
    ]);
    main_box.append(&header_card);

    // 2. Body Text & Extracted Links (ScrolledWindow)
    let body_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    body_box.set_hexpand(true);

    let body_prose = nm_prose_label(&msg.body_text);
    body_box.append(&body_prose);

    if !msg.urls.is_empty() {
        let links_header = nm_field_label("EXTRACTED LINKS");
        body_box.append(&links_header);

        for url in &msg.urls {
            let link_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            let link_icon = nm_icon("external-link", Tone::Accent, 14);
            link_row.append(link_icon.widget());

            let link_lbl = nm_data_label(url);
            link_lbl.set_tone(Tone::Accent);
            link_lbl.set_hexpand(true);
            link_row.append(link_lbl.widget());

            let link_btn = nm_icon_button("external-link", "Open Link");
            let target_url = url.clone();
            link_btn.on_click(move || {
                let _ = Cmd::new("xdg-open").arg(&target_url).spawn_detached();
            });
            link_row.append(link_btn.widget());

            body_box.append(&link_row);
        }
    }

    let scrolled_body = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&body_box)
        .build();
    main_box.append(&scrolled_body);

    // 3. Bottom Reply Drawer Card
    let reply_card_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    let reply_header = nm_field_label("REPLY");
    reply_card_box.append(&reply_header);

    let text_view = gtk4::TextView::new();
    text_view.set_wrap_mode(gtk4::WrapMode::WordChar);
    text_view.set_monospace(false);
    text_view.set_size_request(-1, 100);
    text_view.add_css_class("reply-composer");

    let text_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .height_request(100)
        .child(&text_view)
        .build();
    reply_card_box.append(&text_scrolled);

    let reply_action_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let send_btn = nm_icon_button("send", "Send Reply");
    let reply_status = nm_data_label("");
    reply_status.set_tone(Tone::Secondary);
    reply_status.set_hexpand(true);

    let tv_ref = text_view.clone();
    let to_addr = if !msg.reply_to.is_empty() {
        msg.reply_to.clone()
    } else {
        msg.from.clone()
    };
    let subj_ref = msg.subject.clone();
    let in_reply_to_ref = msg.message_id.clone();
    let refs_ref = msg.references.clone();
    let thread_id_ref = msg.thread_id.clone();
    let tx_send = tx.clone();
    let status_for_send = reply_status.clone();

    send_btn.on_click(move || {
        let buffer = tv_ref.buffer();
        let (start, end) = buffer.bounds();
        let body = buffer.text(&start, &end, false).trim().to_string();

        if body.is_empty() {
            status_for_send.set("CANNOT SEND EMPTY");
            status_for_send.set_tone(Tone::Warning);
            return;
        }

        status_for_send.set("SENDING...");
        status_for_send.set_tone(Tone::Accent);

        let raw_b64 = mime::build_rfc2822_reply(
            &to_addr,
            &subj_ref,
            &in_reply_to_ref,
            &refs_ref,
            &body,
        );

        tx_send.send(EmailMsg::SendReply {
            thread_id: thread_id_ref.clone(),
            raw_b64,
        });
    });

    reply_action_row.append(send_btn.widget());
    reply_action_row.append(reply_status.widget());
    reply_card_box.append(&reply_action_row);

    let reply_card = nm_card([reply_card_box.upcast::<gtk4::Widget>()]);
    main_box.append(&reply_card);

    let tv_focus = text_view.clone();
    reply_btn.on_click(move || {
        tv_focus.grab_focus();
    });

    if focus_reply {
        text_view.grab_focus();
    }

    main_box.upcast::<gtk4::Widget>()
}

fn wait_for_redirect_code(listener: TcpListener) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Nonblocking setup failed: {}", e))?;
    let start = std::time::Instant::now();
    let deadline = Duration::from_secs(120);

    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(conn) => break conn,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() > deadline {
                    return Err("OAuth authorization timed out after 120s".to_string());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Accept failed: {}", e)),
        }
    };

    stream.set_nonblocking(false).ok();
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

    let mut buf = [0u8; 4096];
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

fn build_full_message_cmd(token: &str, msg_id: &str) -> Cmd {
    let url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?format=full",
        msg_id
    );
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
        url, token
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15))
}

fn build_modify_cmd(
    token: &str,
    msg_id: &str,
    add_labels: &[&str],
    remove_labels: &[&str],
) -> Cmd {
    let modify_url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify",
        msg_id
    );
    let add_json = serde_json::to_string(add_labels).unwrap_or_else(|_| "[]".to_string());
    let rem_json = serde_json::to_string(remove_labels).unwrap_or_else(|_| "[]".to_string());
    let body = format!(
        "{{\"addLabelIds\":{},\"removeLabelIds\":{}}}",
        add_json, rem_json
    );

    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\nheader = \"Content-Type: application/json\"\ndata = \"{}\"\n",
        modify_url,
        token,
        body.replace('\\', "\\\\").replace('"', "\\\"")
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "10", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(10))
}

fn build_trash_cmd(token: &str, msg_id: &str) -> Cmd {
    let url = format!(
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/trash",
        msg_id
    );
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\nheader = \"Content-Type: application/json\"\ndata = \"{{}}\"\n",
        url, token
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "10", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(10))
}

fn build_send_cmd(token: &str, raw_b64: &str, thread_id: &str) -> Cmd {
    let url = "https://gmail.googleapis.com/gmail/v1/users/me/messages/send";
    let body = format!(
        "{{\"raw\":\"{}\",\"threadId\":\"{}\"}}",
        raw_b64, thread_id
    );
    let curl_cfg = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\nheader = \"Content-Type: application/json\"\ndata = \"{}\"\n",
        url,
        token,
        body.replace('\\', "\\\\").replace('"', "\\\"")
    );

    Cmd::new("curl")
        .args(["-fsS", "--max-time", "15", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
        .stdin_bytes(curl_cfg.into_bytes())
        .timeout(Duration::from_secs(15))
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
