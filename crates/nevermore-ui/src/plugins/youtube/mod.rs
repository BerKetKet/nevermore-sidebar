use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "youtube",
    "YouTube",
    "youtube",
    7,
    "YouTube search and quick hardware-accelerated playback",
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoItem {
    pub id: String,
    pub duration: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub enum YouTubeMsg {
    SearchResults(Vec<VideoItem>),
    SearchFailed(String),
}

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    // 1. Status & Header Card
    let status_dot = nm_status_dot(Tone::Success);
    let status_label = nm_data_label("READY");
    status_label.set_tone(Tone::Primary);

    let header_toolbar = nm_toolbar(
        [
            status_dot.widget().clone().upcast::<gtk4::Widget>(),
            status_label.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [],
    );
    let header_card = nm_card([header_toolbar.upcast::<gtk4::Widget>()]);

    // 2. Results List Card
    let results_list = nm_list();
    let initial_empty = nm_empty_state("youtube", "Search YouTube videos");
    let initial_row = gtk4::ListBoxRow::new();
    initial_row.set_child(Some(&initial_empty));
    results_list.append(&initial_row);

    let results_card = nm_card([results_list.clone().upcast::<gtk4::Widget>()]);

    // 3. Mailbox Handler
    let list_for_msg = results_list;
    let status_for_msg = status_label.clone();
    let dot_for_msg = status_dot.clone();

    let tx = ctx.mailbox(move |msg: YouTubeMsg| match msg {
        YouTubeMsg::SearchResults(items) => {
            status_for_msg.set("RESULTS");
            status_for_msg.set_tone(Tone::Success);
            dot_for_msg.set_tone(Tone::Success);

            while let Some(child) = list_for_msg.first_child() {
                list_for_msg.remove(&child);
            }
            if items.is_empty() {
                let empty = nm_empty_state("youtube", "No matching videos found");
                let row = gtk4::ListBoxRow::new();
                row.set_child(Some(&empty));
                list_for_msg.append(&row);
            } else {
                for item in items {
                    let play_btn = nm_icon_button("play", "Play video with mpv");
                    let vid = item.id.clone();
                    play_btn.on_click(move || {
                        let url = format!("https://www.youtube.com/watch?v={}", vid);
                        let cmd = Cmd::new("mpv")
                            .arg("--hwdec=vaapi")
                            .arg("--vo=gpu")
                            .arg("--ytdl-format=bestvideo[height<=1080]+bestaudio/best")
                            .arg(&url);
                        let _ = cmd.execute_blocking();
                    });

                    let display_title = if item.title.len() > 42 {
                        format!("{}...", &item.title[..39])
                    } else {
                        item.title.clone()
                    };

                    let row = nm_list_row(
                        "youtube",
                        &display_title,
                        Some(&item.duration),
                        [play_btn.widget().clone().upcast::<gtk4::Widget>()],
                    );
                    list_for_msg.append(&row);
                }
            }
        }
        YouTubeMsg::SearchFailed(err) => {
            status_for_msg.set(&err);
            status_for_msg.set_tone(Tone::Danger);
            dot_for_msg.set_tone(Tone::Danger);
        }
    });

    // 4. Search Entry
    let search_tx = tx;
    let status_for_search = status_label;
    let dot_for_search = status_dot;
    let ctx_run = ctx.clone();

    let search_entry = nm_search_entry("Search YouTube videos", move |query| {
        if query.trim().is_empty() {
            return;
        }
        let q = query.trim().to_string();
        status_for_search.set("SEARCHING");
        status_for_search.set_tone(Tone::Accent);
        dot_for_search.set_tone(Tone::Accent);

        let search_arg = format!("ytsearch6:{}", q);
        let cmd = Cmd::new("yt-dlp")
            .arg("--flat-playlist")
            .arg("--print")
            .arg("%(id)s\t%(duration_string)s\t%(title)s")
            .arg(&search_arg)
            .timeout(Duration::from_secs(10));

        let tx_inner = search_tx.clone();
        ctx_run.run(cmd, tx_inner, |res| {
            if res.code != Some(0) {
                return YouTubeMsg::SearchFailed("SEARCH FAILED".to_string());
            }
            let mut items = Vec::new();
            for line in res.stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    items.push(VideoItem {
                        id: parts[0].to_string(),
                        duration: parts[1].to_string(),
                        title: parts[2].to_string(),
                    });
                }
            }
            YouTubeMsg::SearchResults(items)
        });
    });

    // 5. Clamped Vertical Page
    let page = nm_page([
        header_card.upcast::<gtk4::Widget>(),
        search_entry.upcast::<gtk4::Widget>(),
        results_card.upcast::<gtk4::Widget>(),
    ]);
    page.upcast::<gtk4::Widget>()
}
