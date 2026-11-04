use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "gemini",
    "Gemini Chat",
    "gemini",
    6,
    "Native Gemini AI chat interface with history",
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
    ReplyReceived { prompt: String, reply: String },
    ErrorOccurred(String),
    HistoryCleared,
}

fn get_api_key(ctx: &PluginContext) -> Option<String> {
    if let Some(key) = ctx.secret_get_blocking("gemini_api_key") {
        if !key.trim().is_empty() {
            return Some(key.trim().to_string());
        }
    }
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.trim().is_empty() {
            return Some(key.trim().to_string());
        }
    }
    None
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let api_key = get_api_key(ctx);
    let history: ChatHistory = ctx.load_json("chat_history");

    // 1. Status & Header Card
    let status_dot = nm_status_dot(if api_key.is_some() {
        Tone::Success
    } else {
        Tone::Warning
    });

    let status_label = nm_data_label(if api_key.is_some() {
        "ONLINE"
    } else {
        "KEY MISSING"
    });
    status_label.set_tone(if api_key.is_some() {
        Tone::Success
    } else {
        Tone::Warning
    });

    let clear_btn = nm_icon_button("eraser", "Clear conversation history");

    let header_toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            status_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [clear_btn.widget().clone().upcast::<gtk4::Widget>()],
    );
    let header_card = nm_card([header_toolbar.upcast::<gtk4::Widget>()]);

    // 2. Chat List Container
    let chat_list = nm_list();
    let chat_card = nm_card([chat_list.clone().upcast::<gtk4::Widget>()]);

    // Render initial history
    let render_turns = |list: &gtk4::ListBox, turns: &[ChatTurn]| {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        if turns.is_empty() {
            let empty = nm_empty_state("gemini", "No conversation history");
            let row = gtk4::ListBoxRow::new();
            row.set_child(Some(&empty));
            list.append(&row);
        } else {
            for turn in turns.iter().rev().take(6) {
                let icon_name = if turn.role == "user" {
                    "terminal"
                } else {
                    "sparkles"
                };
                let role_tag = if turn.role == "user" {
                    "USER"
                } else {
                    "MODEL"
                };
                let display_text = if turn.content.len() > 60 {
                    format!("{}...", &turn.content[..57])
                } else {
                    turn.content.clone()
                };
                let item = nm_list_row(icon_name, &display_text, Some(role_tag), []);
                list.append(&item);
            }
        }
    };

    render_turns(&chat_list, &history.turns);

    // 3. Mailbox Handler
    let list_for_msg = chat_list.clone();
    let status_for_msg = status_label.clone();
    let ctx_save = ctx.clone();

    let tx = ctx.mailbox(move |msg: GeminiMsg, _tx| match msg {
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
            render_turns(&list_for_msg, &cur_history.turns);
        }
        GeminiMsg::ErrorOccurred(err) => {
            status_for_msg.set(&err);
            status_for_msg.set_tone(Tone::Danger);
        }
        GeminiMsg::HistoryCleared => {
            let empty_hist = ChatHistory::default();
            let _ = ctx_save.save_json("chat_history", &empty_hist);
            render_turns(&list_for_msg, &empty_hist.turns);
            status_for_msg.set("CLEARED");
            status_for_msg.set_tone(Tone::Muted);
        }
    });

    // Clear Button Handler
    let clear_tx = tx.clone();
    clear_btn.on_click(move || {
        clear_tx.send(GeminiMsg::HistoryCleared);
    });

    // 4. Prompt Search / Entry Bar
    let prompt_tx = tx;
    let status_for_prompt = status_label;
    let ctx_run = ctx.clone();

    let prompt_entry = nm_search_entry("Prompt Gemini AI assistant", move |query| {
        if query.trim().is_empty() {
            return;
        }
        let q = query.trim().to_string();
        status_for_prompt.set("INFERENCE");
        status_for_prompt.set_tone(Tone::Accent);

        let key_opt = get_api_key(&ctx_run);
        if key_opt.is_none() {
            prompt_tx.send(GeminiMsg::ErrorOccurred("KEY REQUIRED".to_string()));
            return;
        }
        let key = key_opt.unwrap();

        let escaped_prompt = serde_json::to_string(&q).unwrap_or_else(|_| "\"\"".to_string());
        let body = format!("{{\"contents\":[{{\"parts\":[{{\"text\":{}}}]}}]}}", escaped_prompt);
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            key
        );

        let cmd = Cmd::curl(&url)
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(&body)
            .timeout(Duration::from_secs(15));

        let tx_inner = prompt_tx.clone();
        let prompt_record = q.clone();
        ctx_run.run(cmd, &tx_inner, move |res| {
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
                    let err_short = if err_msg.len() > 20 {
                        &err_msg[..20]
                    } else {
                        err_msg
                    };
                    return GeminiMsg::ErrorOccurred(err_short.to_uppercase());
                }
            }
            GeminiMsg::ErrorOccurred("PARSE ERROR".to_string())
        });
    });

    // 5. Clamped Vertical Page
    let page = nm_page([
        header_card.upcast::<gtk4::Widget>(),
        prompt_entry.upcast::<gtk4::Widget>(),
        chat_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
