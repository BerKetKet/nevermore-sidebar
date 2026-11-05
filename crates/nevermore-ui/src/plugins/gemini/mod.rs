use nevermore_ui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub const GEMINI_MODELS: &[&str] = &[
    "gemini-2.5-flash",
    "gemini-3-flash-preview",
    "gemini-3.1-flash-lite",
    "gemini-2.5-pro",
    "gemini-2.0-flash",
];

pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new(
        "gemini_api_key",
        "Gemini API Key",
        SettingKind::Secret {
            key_name: "gemini_api_key",
        },
    ),
    SettingSpec::new(
        "model",
        "Gemini Model",
        SettingKind::Choice {
            options: GEMINI_MODELS,
            default: "gemini-3-flash-preview",
        },
    ),
];

pub const SPEC: PluginSpec = PluginSpec::with_settings(
    "gemini",
    "Gemini Chat",
    "gemini",
    6,
    "Native Gemini AI chat interface with history",
    SETTINGS,
    None,
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
    pub timestamp_sec: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChatHistory {
    pub turns: Vec<ChatTurn>,
}

#[derive(Clone, Debug)]
pub enum GeminiMsg {
    KeyChecked(Option<String>),
    ReplyReceived { prompt: String, reply: String },
    ErrorOccurred(String),
    HistoryCleared,
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let initial_key = std::env::var("GEMINI_API_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty());
    let cached_key: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(initial_key.clone()));
    let history: ChatHistory = ctx.load_json("chat_history");

    // 1. Status & Header Card
    let status_dot = nm_status_dot(if initial_key.is_some() {
        Tone::Success
    } else {
        Tone::Warning
    });

    let status_label = nm_data_label(if initial_key.is_some() {
        "ONLINE"
    } else {
        "KEY MISSING"
    });
    status_label.set_tone(if initial_key.is_some() {
        Tone::Success
    } else {
        Tone::Warning
    });

    let clear_btn = nm_icon_button("trash-2", "Clear conversation history");

    let header_toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            status_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [clear_btn.widget().clone().upcast::<gtk4::Widget>()],
    );
    let header_card = nm_card([header_toolbar.upcast::<gtk4::Widget>()]);

    // 2. Chat Turns Container
    let chat_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    chat_box.set_hexpand(true);

    let render_turns = |container: &gtk4::Box, turns: &[ChatTurn]| {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }

        if turns.is_empty() {
            let empty = nm_empty_state("gemini", "No conversation history");
            container.append(&empty);
        } else {
            // Natural reading order: display turns chronologically, showing the last 10 turns
            let start_idx = turns.len().saturating_sub(10);
            for turn in &turns[start_idx..] {
                let is_user = turn.role == "user";
                let icon_name = if is_user { "terminal" } else { "gemini" };
                let turn_icon = nm_icon(icon_name, if is_user { Tone::Secondary } else { Tone::Accent }, 16);

                let prose = nm_prose_label(&turn.content);

                let header_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                header_row.append(turn_icon.widget());

                let role_label = nm_field_label(if is_user { "PROMPT" } else { "GEMINI" });
                header_row.append(&role_label);

                let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
                spacer.set_hexpand(true);
                header_row.append(&spacer);

                if !is_user {
                    let copy_btn = nm_icon_button("copy", "Copy response to clipboard");
                    let content_to_copy = turn.content.clone();
                    copy_btn.on_click(move || {
                        if let Some(display) = gtk4::gdk::Display::default() {
                            display.clipboard().set_text(&content_to_copy);
                        }
                    });
                    header_row.append(copy_btn.widget());
                }

                let turn_card = nm_card([
                    header_row.upcast::<gtk4::Widget>(),
                    prose.upcast::<gtk4::Widget>(),
                ]);
                container.append(&turn_card);
            }
        }
    };

    render_turns(&chat_box, &history.turns);

    // 3. Mailbox Handler
    let box_for_msg = chat_box.clone();
    let status_for_msg = status_label.clone();
    let status_dot_for_msg = status_dot;
    let cached_key_for_msg = cached_key.clone();
    let ctx_save = ctx.clone();

    let tx = ctx.mailbox(move |msg: GeminiMsg, _tx| match msg {
        GeminiMsg::KeyChecked(key) => {
            let has_key = key.is_some();
            *cached_key_for_msg.borrow_mut() = key;
            status_dot_for_msg.set_tone(if has_key {
                Tone::Success
            } else {
                Tone::Warning
            });
            status_for_msg.set(if has_key { "ONLINE" } else { "KEY MISSING" });
            status_for_msg.set_tone(if has_key {
                Tone::Success
            } else {
                Tone::Warning
            });
        }
        GeminiMsg::ReplyReceived { prompt, reply } => {
            status_for_msg.set("READY");
            status_for_msg.set_tone(Tone::Success);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let mut cur_history: ChatHistory = ctx_save.load_json("chat_history");
            cur_history.turns.push(ChatTurn {
                role: "user".to_string(),
                content: prompt,
                timestamp_sec: now,
            });
            cur_history.turns.push(ChatTurn {
                role: "model".to_string(),
                content: reply,
                timestamp_sec: now,
            });

            let _ = ctx_save.save_json("chat_history", &cur_history);
            render_turns(&box_for_msg, &cur_history.turns);
        }
        GeminiMsg::ErrorOccurred(err) => {
            status_for_msg.set(&err);
            status_for_msg.set_tone(Tone::Danger);
        }
        GeminiMsg::HistoryCleared => {
            let empty_hist = ChatHistory::default();
            let _ = ctx_save.save_json("chat_history", &empty_hist);
            render_turns(&box_for_msg, &empty_hist.turns);
            status_for_msg.set("CLEARED");
            status_for_msg.set_tone(Tone::Muted);
        }
    });

    // Clear Button Handler
    let clear_tx = tx.clone();
    clear_btn.on_click(move || {
        clear_tx.send(GeminiMsg::HistoryCleared);
    });

    // 4. Prompt Entry
    let prompt_tx = tx.clone();
    let status_for_prompt = status_label.clone();
    let ctx_submit = ctx.clone();
    let cached_key_for_submit = cached_key;

    let prompt_entry = nm_prompt_entry("gemini", "Prompt Gemini AI assistant...", move |query| {
        let q = query.trim().to_string();
        if q.is_empty() {
            return;
        }

        status_for_prompt.set("INFERENCE");
        status_for_prompt.set_tone(Tone::Accent);

        let key = match cached_key_for_submit.borrow().clone() {
            Some(k) if !k.is_empty() => k,
            _ => {
                prompt_tx.send(GeminiMsg::ErrorOccurred("KEY REQUIRED".to_string()));
                return;
            }
        };

        let model = ctx_submit
            .setting_str("model")
            .unwrap_or_else(|| "gemini-3-flash-preview".to_string());

        let escaped_prompt = serde_json::to_string(&q).unwrap_or_else(|_| "\"\"".to_string());
        let body = format!("{{\"contents\":[{{\"parts\":[{{\"text\":{}}}]}}]}}", escaped_prompt);

        let curl_config = format!(
            "url = \"https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent\"\nheader = \"x-goog-api-key: {}\"\nheader = \"Content-Type: application/json\"\ndata = \"{}\"\n",
            model,
            key,
            body.replace('\\', "\\\\").replace('"', "\\\"")
        );

        let cmd = Cmd::new("curl")
            .args(["-fsS", "--max-time", "20", "--proto", "=https", "-A", "Nevermore/0.1", "-K", "-"])
            .stdin_bytes(curl_config.into_bytes())
            .timeout(Duration::from_secs(20));

        let tx_inner = prompt_tx.clone();
        let prompt_record = q.clone();
        ctx_submit.run(cmd, &tx_inner, move |res| {
            if res.code != Some(0) {
                return GeminiMsg::ErrorOccurred("NETWORK ERROR".to_string());
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&res.stdout) {
                if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                    return GeminiMsg::ReplyReceived {
                        prompt: prompt_record,
                        reply: text.trim().to_string(),
                    };
                }
                if let Some(err_msg) = json["error"]["message"].as_str() {
                    let err_short: String = err_msg.chars().take(20).collect();
                    return GeminiMsg::ErrorOccurred(err_short.to_uppercase());
                }
            }
            GeminiMsg::ErrorOccurred("PARSE ERROR".to_string())
        });
    });

    // 5. On Show: check API key in background thread
    let tx_show = tx;
    let plugin_id = ctx.plugin_id.clone();
    ctx.on_show(move || {
        let tx_bg = tx_show.clone();
        let pid = plugin_id.clone();
        std::thread::spawn(move || {
            let key = nevermore_ui::plugin_api::secret_lookup(&pid, "gemini_api_key")
                .or_else(|| std::env::var("GEMINI_API_KEY").ok())
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty());
            tx_bg.send(GeminiMsg::KeyChecked(key));
        });
    });

    // 6. Clamped Vertical Page
    let page = nm_page([
        header_card.upcast::<gtk4::Widget>(),
        prompt_entry.upcast::<gtk4::Widget>(),
        chat_box.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
