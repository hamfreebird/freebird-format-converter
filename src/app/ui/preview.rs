use crate::core::player::mpv::PlaybackState;

/// 渲染视频预览面板
pub fn render_preview_panel(
    ctx: &egui::Context,
    show: &mut bool,
    state: &PlaybackState,
    is_playing: bool,
    on_play_pause: impl FnMut(),
    on_stop: impl FnMut(),
    on_seek: impl FnMut(f64),       // 跳转到绝对秒数
    on_volume: impl FnMut(f64),     // 设置音量 0-100
    on_screenshot: impl FnMut(),
) {
    if !*show {
        return;
    }

    let mut window_open = true;

    egui::Window::new("Preview")
        .open(&mut window_open)
        .resizable(true)
        .collapsible(true)
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .min_width(360.0)
        .default_height(200.0)
        .show(ctx, |ui| {
            render_controls(
                ui,
                state,
                is_playing,
                on_play_pause,
                on_stop,
                on_seek,
                on_volume,
                on_screenshot,
            );
        });

    if !window_open {
        *show = false;
    }
}

fn render_controls(
    ui: &mut egui::Ui,
    state: &PlaybackState,
    is_playing: bool,
    mut on_play_pause: impl FnMut(),
    mut on_stop: impl FnMut(),
    mut on_seek: impl FnMut(f64),
    mut on_volume: impl FnMut(f64),
    mut on_screenshot: impl FnMut(),
) {
    // ── 文件信息 ──
    if let Some(ref filename) = state.filename {
        ui.label(filename);
    } else {
        ui.label("No file loaded");
    }
    ui.add_space(4.0);

    // ── 进度条 ──
    let progress = state.progress_percent() as f64 / 100.0;
    let mut seek_pos = progress;
    let seek_response = ui.add(
        egui::Slider::new(&mut seek_pos, 0.0..=1.0)
            .text("")
            .show_value(false),
    );
    if seek_response.changed() {
        if let Some(dur) = state.duration {
            on_seek(seek_pos * dur);
        }
    }

    // ── 时间显示 ──
    ui.horizontal(|ui| {
        ui.label(state.time_display());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(state.duration_display());
        });
    });

    ui.add_space(4.0);

    // ── 播放控制按钮 ──
    ui.horizontal(|ui| {
        ui.centered_and_justified(|ui| {
            // 停止
            if ui.button("Stop").clicked() {
                on_stop();
            }

            // 快退 10 秒
            if ui.button("<<").clicked() {
                if let Some(pos) = state.time_pos {
                    on_seek((pos - 10.0).max(0.0));
                }
            }

            // 播放/暂停
            let play_label = if state.paused || !is_playing { "Play" } else { "Pause" };
            if ui.button(play_label).clicked() {
                on_play_pause();
            }

            // 快进 10 秒
            if ui.button(">>").clicked() {
                if let (Some(pos), Some(dur)) = (state.time_pos, state.duration) {
                    on_seek((pos + 10.0).min(dur));
                }
            }

            // 截图
            if ui.button("Shot").clicked() {
                on_screenshot();
            }
        });
    });

    ui.add_space(4.0);

    // ── 音量控制 ──
    ui.horizontal(|ui| {
        ui.label("Vol");
        let mut vol = state.volume;
        let vol_response = ui.add(
            egui::Slider::new(&mut vol, 0.0..=100.0)
                .text("")
                .show_value(true),
        );
        if vol_response.changed() {
            on_volume(vol);
        }
    });

    // ── 详细状态信息 ──
    ui.add_space(4.0);
    ui.collapsing("Details", |ui| {
        ui.label(format!("Paused: {}", state.paused));
        ui.label(format!("Volume: {:.0}%", state.volume));
        ui.label(format!("Speed: {:.2}x", state.speed));
        ui.label(format!("Muted: {}", state.muted));
    });
}
