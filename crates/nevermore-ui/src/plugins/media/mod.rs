use nevermore_ui::prelude::*;

pub const SPEC: PluginSpec = PluginSpec::new(
    "media",
    "Media Player",
    "media",
    5,
    "Media player playback and controls",
);

pub fn build(ctx: &PluginContext) -> gtk4::Widget {
    let title_label = nm_data_label("STOPPED");
    title_label.set_tone(Tone::Primary);

    let progress = nm_metric_row("clock", Tone::Accent);
    progress.set(0.0, "0:00 / 0:00");

    let prev_btn = nm_icon_button("chevron-left", "Previous track");
    let title_prev = title_label.clone();
    prev_btn.on_click(move || {
        title_prev.set("PREV");
    });

    let play_toggle = nm_toggle_icon_button("play", "pause", "Play or pause");
    let title_play = title_label.clone();
    play_toggle.on_toggle(move |active| {
        if active {
            title_play.set("PLAYING");
        } else {
            title_play.set("PAUSED");
        }
    });

    let next_btn = nm_icon_button("chevron-right", "Next track");
    let title_next = title_label.clone();
    next_btn.on_click(move || {
        title_next.set("NEXT");
    });

    let controls = nm_toolbar(
        [
            prev_btn.widget().clone().upcast::<gtk4::Widget>(),
            play_toggle.widget().clone().upcast::<gtk4::Widget>(),
            next_btn.widget().clone().upcast::<gtk4::Widget>(),
        ],
        [],
    );

    let track_card = nm_card([
        title_label.widget().clone().upcast::<gtk4::Widget>(),
        progress.widget().clone().upcast::<gtk4::Widget>(),
        controls.upcast::<gtk4::Widget>(),
    ]);

    let title_clone = title_label.clone();
    let progress_clone = progress.clone();
    let tx = ctx.mailbox(move |status: String| {
        title_clone.set(&status);
        if status == "PLAYING" {
            progress_clone.set(0.45, "1:42 / 3:45");
        } else {
            progress_clone.set(0.0, "0:00 / 0:00");
        }
    });

    let tx_check = tx.clone();
    ctx.every(Duration::from_secs(3), {
        let tx_check = tx_check.clone();
        move || {
            let cmd = Cmd::new("playerctl")
                .arg("status")
                .timeout(Duration::from_millis(500));
            std::thread::spawn({
                let tx = tx_check.clone();
                move || {
                    let res = cmd.execute_blocking();
                    if res.stdout.trim() == "Playing" {
                        tx.send("PLAYING".to_string());
                    } else if res.stdout.trim() == "Paused" {
                        tx.send("PAUSED".to_string());
                    } else {
                        tx.send("STOPPED".to_string());
                    }
                }
            });
        }
    });

    let page = nm_page([track_card.upcast::<gtk4::Widget>()]);
    page.upcast::<gtk4::Widget>()
}
