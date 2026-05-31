/// 关于对话框
pub fn show_about_dialog(ctx: &egui::Context, show: &mut bool) {
    if !*show {
        return;
    }

    let mut window_open = true;
    let mut close_clicked = false;

    egui::Window::new("About freebird format converter")
        .open(&mut window_open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(320.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("freebird format converter");
                ui.label("v0.1.0");
                ui.add_space(8.0);
                ui.label("A convenient media file format converter");
                ui.label("based on FFmpeg.");
                ui.add_space(12.0);
                ui.label("Author: hamfreebird");
                ui.label("License: GPL-3.0");
                ui.add_space(8.0);
                ui.hyperlink_to(
                    "Source code",
                    "https://github.com/hamfreebird/freebird-format-converter",
                );
                ui.add_space(12.0);
                if ui.button("OK").clicked() {
                    close_clicked = true;
                }
            });
        });

    if !window_open || close_clicked {
        *show = false;
    }
}

/// 确认覆盖对话框
/// 返回 true = 确认覆盖, false = 取消, None = 仍在显示
pub fn show_overwrite_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    filename: &str,
) -> Option<bool> {
    if !*show {
        return None;
    }

    let mut result = None;
    let mut window_open = true;

    egui::Window::new("File already exists")
        .open(&mut window_open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(350.0)
        .show(ctx, |ui| {
            ui.label(format!(
                "The output file \"{}\" already exists.\nDo you want to overwrite it?",
                filename
            ));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Overwrite").clicked() {
                        result = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        result = Some(false);
                    }
                });
            });
        });

    // 如果点击了按钮（result 被设置）或窗口被 X 关闭
    if result.is_some() || !window_open {
        *show = false;
        Some(result.unwrap_or(false))
    } else {
        None
    }
}

/// 错误对话框
pub fn show_error_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    title: &str,
    message: &str,
) {
    if !*show {
        return;
    }

    let mut window_open = true;
    let mut close_clicked = false;

    egui::Window::new(title.to_string())
        .open(&mut window_open)
        .resizable(true)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(350.0)
        .max_height(400.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.monospace(message);
            });
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("OK").clicked() {
                    close_clicked = true;
                }
            });
        });

    if !window_open || close_clicked {
        *show = false;
    }
}

/// FFmpeg 版本信息对话框
pub fn show_ffmpeg_version_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    version_text: &str,
) {
    if !*show {
        return;
    }

    let mut window_open = true;
    let mut close_clicked = false;

    egui::Window::new("FFmpeg Version")
        .open(&mut window_open)
        .resizable(true)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(500.0)
        .max_height(500.0)
        .show(ctx, |ui| {
            ui.heading("FFmpeg Version Information");
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.monospace(version_text);
                });
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("OK").clicked() {
                    close_clicked = true;
                }
            });
        });

    if !window_open || close_clicked {
        *show = false;
    }
}
