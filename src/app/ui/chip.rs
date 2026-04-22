use egui::Ui;

pub fn render_chip_window(
    ui: &mut Ui,
    error_message: Option<String>,) {
    egui::Frame::default()
        .inner_margin(egui::Margin::same(10i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Log: ");
                // 显示运行状态
                ui.label(error_message
                    .as_deref()                     // Option<&str>
                    .unwrap_or("As right as rain~  ヾ(≧▽≦*)o")           // &str
                    .to_string());
            });

            ui.separator();

            // TODO: 渲染切片页面的UI和对应逻辑
        });
}