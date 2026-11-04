use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "gemini",
    "Gemini Chat",
    "gemini",
    6,
    "Gemini AI assistant prompt interface",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let status_dot = nm_status_dot(Tone::Success);
    let status_label = nm_data_label("ONLINE");
    status_label.set_tone(Tone::Success);

    let header_card = nm_card([
        status_dot.widget().clone().upcast::<gtk4::Widget>(),
        status_label.widget().clone().upcast::<gtk4::Widget>(),
    ]);

    let status_query = status_label.clone();
    let prompt_entry = nm_search_entry("Prompt Gemini assistant", move |query| {
        if !query.is_empty() {
            status_query.set("QUERYING");
        } else {
            status_query.set("ONLINE");
        }
    });

    let row1 = nm_list_row("sparkles", "PROMPT 01", Some("COMPLETED"), []);
    let row2 = nm_list_row("sparkles", "SYSTEM", Some("READY"), []);
    let history_list = nm_list();
    history_list.append(&row1);
    history_list.append(&row2);
    let history_card = nm_card([history_list.upcast::<gtk4::Widget>()]);

    ctx.every(Duration::from_secs(15), {
        let status_dot = status_dot.clone();
        move || {
            status_dot.set_tone(Tone::Success);
        }
    });

    let page = nm_page([
        header_card.upcast::<gtk4::Widget>(),
        prompt_entry.upcast::<gtk4::Widget>(),
        history_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
