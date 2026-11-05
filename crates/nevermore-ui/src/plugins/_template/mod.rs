use nevermore_ui::prelude::*;

pub const SETTINGS: &[SettingSpec] = &[
    SettingSpec::new(
        "refresh_interval",
        "Refresh Interval (s)",
        SettingKind::Int {
            default: 60,
            min: 5,
            max: 3600,
        },
    ),
    SettingSpec::new(
        "auto_refresh",
        "Auto Refresh",
        SettingKind::Bool { default: true },
    ),
    SettingSpec::new(
        "api_token",
        "API Token",
        SettingKind::Secret {
            key_name: "api_token",
        },
    ),
];

pub const SPEC: PluginSpec = PluginSpec::with_settings(
    "template",
    "Template Plugin",
    "terminal",
    99,
    "Authoritative reference implementation for Nevermore plugins",
    SETTINGS,
    None,
);

enum TemplateMsg {
    PromptSubmitted(String),
    TimerFired,
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Section Header & Field Label
    let header = nm_section_header("TEMPLATE CONTROLS");
    let status_label = nm_field_label("Current Status");

    // 2. Data-only status label
    let status = nm_data_label("IDLE");
    status.set_tone(Tone::Muted);

    // 3. Multi-line Prose Label
    let prose = nm_prose_label("Type a message below and press Enter to see it reflected.");

    // 4. Mailbox & Message Handling
    let status_clone = status.clone();
    let prose_clone = prose.clone();
    let tx = ctx.mailbox(move |msg: TemplateMsg, _tx| match msg {
        TemplateMsg::PromptSubmitted(text) => {
            status_clone.set("ACTIVE");
            status_clone.set_tone(Tone::Accent);
            prose_clone.set_text(&format!("Last prompt: {}", text));
        }
        TemplateMsg::TimerFired => {
            status_clone.set("PING");
            status_clone.set_tone(Tone::Secondary);
        }
    });

    // 5. Prompt Entry (activates only on Enter / send button)
    let tx_submit = tx.clone();
    let prompt = nm_prompt_entry("terminal", "Enter text...", move |query| {
        tx_submit.send(TemplateMsg::PromptSubmitted(query));
    });

    // 6. Action Button
    let btn = nm_icon_button("check", "Execute action");
    let tx_btn = tx.clone();
    btn.on_click(move || {
        tx_btn.send(TemplateMsg::TimerFired);
    });

    let card = nm_card([
        header.upcast::<gtk4::Widget>(),
        status_label.upcast::<gtk4::Widget>(),
        status.widget().clone().upcast::<gtk4::Widget>(),
        prose.upcast::<gtk4::Widget>(),
        prompt.upcast::<gtk4::Widget>(),
        btn.widget().clone().upcast::<gtk4::Widget>(),
    ]);

    // 7. Periodic timer (automatically paused when tab is hidden)
    let tx_timer = tx;
    ctx.every(Duration::from_secs(10), move || {
        tx_timer.send(TemplateMsg::TimerFired);
    });

    let page = nm_page([card.upcast::<gtk4::Widget>()]);
    page.upcast::<gtk4::Widget>()
}
