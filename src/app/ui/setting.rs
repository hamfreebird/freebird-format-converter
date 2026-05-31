use crate::core::utils::AppSettings;
use egui::Ui;

/// 渲染设置对话框（作为 egui::Window 模态窗口）
pub fn render_settings_dialog(
    ctx: &egui::Context,
    settings: &mut AppSettings,
    show: &mut bool,
) {
    if !*show {
        return;
    }

    let mut window_open = true;
    let mut close_requested = false;

    egui::Window::new("Settings")
        .open(&mut window_open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(400.0)
        .show(ctx, |ui| {
            render_settings_content(ui, settings, &mut close_requested);
        });

    // X 按钮或内部按钮触发关闭
    if !window_open || close_requested {
        *show = false;
        if let Err(e) = settings.save() {
            eprintln!("Failed to save settings: {}", e);
        }
    }
}

fn render_settings_content(ui: &mut Ui, settings: &mut AppSettings, close_requested: &mut bool) {
    ui.heading("Task Settings");

    ui.add_space(8.0);

    // 最大并发任务数
    ui.horizontal(|ui| {
        ui.label("Max concurrent tasks:");
        ui.add(egui::Slider::new(&mut settings.max_concurrent_tasks, 1..=16)
            .step_by(1.0)
            .text("tasks"));
    });
    ui.label(format!(
        "Current: {} task(s) running simultaneously",
        settings.max_concurrent_tasks
    ));

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);

    ui.heading("Paths");

    // 默认输出目录
    ui.horizontal(|ui| {
        ui.label("Default output directory:");
        let dir_text = settings
            .default_output_dir
            .as_deref()
            .unwrap_or("(same as input file)");
        ui.add(
            egui::TextEdit::singleline(&mut dir_text.to_string())
                .desired_width(200.0)
                .interactive(false),
        );
    });

    ui.add_space(8.0);

    // FFmpeg 路径
    ui.horizontal(|ui| {
        ui.label("FFmpeg path:");
        let ffmpeg_text = settings
            .ffmpeg_path
            .as_deref()
            .unwrap_or("(auto-detect from PATH)");
        ui.add(
            egui::TextEdit::singleline(&mut ffmpeg_text.to_string())
                .desired_width(200.0)
                .interactive(false),
        );
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // 按钮行
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Save & Close").clicked() {
                if let Err(e) = settings.save() {
                    eprintln!("Failed to save settings: {}", e);
                }
                *close_requested = true;
            }
            if ui.button("Cancel").clicked() {
                // 恢复之前保存的设置
                let saved = AppSettings::load();
                *settings = saved;
                *close_requested = true;
            }
        });
    });

    ui.add_space(4.0);
    ui.label("Settings are automatically saved to:");
    ui.monospace(
        dirs_next()
            .unwrap_or_else(|| "unknown".into())
            .to_string_lossy()
            .to_string()
            + "/freebird-format-converter/settings.json",
    );
}

/// 获取用户配置目录（与 config.rs 保持一致）
#[cfg(not(target_os = "windows"))]
fn dirs_next() -> Option<std::path::PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
}

#[cfg(target_os = "windows")]
fn dirs_next() -> Option<std::path::PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(std::path::PathBuf::from)
}
