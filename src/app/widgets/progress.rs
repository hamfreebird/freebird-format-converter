use egui::{Color32, Rect, Response, Ui, Vec2};

/// 作业状态徽章
pub fn status_badge(ui: &mut Ui, status: &str) -> Response {
    let (bg_color, text) = match status {
        "Pending" => (Color32::from_rgb(100, 100, 100), "Pending"),
        "Running" => (Color32::from_rgb(59, 130, 246), "Running"),
        "Completed" => (Color32::from_rgb(34, 197, 94), "Completed"),
        "Failed" => (Color32::from_rgb(239, 68, 68), "Failed"),
        "Cancelled" => (Color32::from_rgb(251, 191, 36), "Cancelled"),
        _ => (Color32::from_rgb(150, 150, 150), status),
    };

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(90.0, 18.0),
        egui::Sense::hover(),
    );

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(4),
            bg_color,
        );

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
    }

    response
}

/// 渐变色进度条
pub fn progress_bar(ui: &mut Ui, percent: f32) -> Response {
    let desired_size = Vec2::new(ui.available_width(), 14.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // 背景
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(4),
            Color32::from_rgb(40, 40, 40),
        );

        // 进度填充
        let clamped = percent.clamp(0.0, 100.0) / 100.0;
        if clamped > 0.0 {
            let fill_width = rect.width() * clamped;
            let fill_rect = Rect::from_min_size(
                rect.min,
                Vec2::new(fill_width, rect.height()),
            );

            // 渐变色
            let fill_color = if clamped < 0.5 {
                let t = clamped * 2.0;
                Color32::from_rgb(
                    (59.0 + (40.0 - 59.0) * t) as u8,
                    (130.0 + (40.0 - 130.0) * t) as u8,
                    (246.0 + (40.0 - 246.0) * t) as u8,
                )
            } else {
                let t = (clamped - 0.5) * 2.0;
                Color32::from_rgb(
                    (59.0 + (34.0 - 59.0) * t) as u8,
                    (130.0 + (197.0 - 130.0) * t) as u8,
                    (246.0 + (94.0 - 246.0) * t) as u8,
                )
            };

            painter.rect_filled(
                fill_rect,
                egui::CornerRadius::same(4),
                fill_color,
            );
        }

        // 进度文本
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}%", clamped * 100.0),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }

    response
}

/// 耗时显示（如 "00:05:23"）
pub fn elapsed_display(ui: &mut Ui, elapsed_secs: Option<f64>) {
    let text = match elapsed_secs {
        Some(s) if s > 0.0 => {
            let h = (s / 3600.0) as u64;
            let m = ((s % 3600.0) / 60.0) as u64;
            let sec = (s % 60.0) as u64;
            format!("{:02}:{:02}:{:02}", h, m, sec)
        }
        _ => "--:--:--".to_string(),
    };
    ui.label(text);
}

/// 带标签的进度行（文件名 + 状态徽章 + 进度条 + 耗时）
pub fn job_progress_row(
    ui: &mut Ui,
    filename: &str,
    status: &str,
    percent: f32,
    elapsed_secs: Option<f64>,
) {
    ui.horizontal(|ui| {
        // 文件名（截断）
        let display_name = if filename.chars().count() > 30 {
            format!(
                "{}…",
                filename.chars().take(27).collect::<String>()
            )
        } else {
            filename.to_string()
        };
        ui.label(display_name);

        ui.add_space(4.0);

        // 状态徽章
        status_badge(ui, status);

        ui.add_space(4.0);

        // 进度条
        progress_bar(ui, percent);

        ui.add_space(4.0);

        // 耗时
        elapsed_display(ui, elapsed_secs);
    });
}
