use crate::channels::messages::UiMessages;
use egui_inbox::UiInboxSender;

/// 渲染顶部菜单栏
#[allow(deprecated)]
pub fn render_menu_bar(
    ctx: &egui::Context,
    _sender: &UiInboxSender<UiMessages>,
) {
    // 使用 Panel::top 替代已弃用的 TopBottomPanel
    egui::Panel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // ── File 菜单 ──
            ui.menu_button("File", |ui| {
                if ui.button("📂 Add File...").clicked() {
                    let _ = _sender.send(UiMessages::PickFile(None));
                    ui.close_menu();
                }
                if ui.button("📁 Add Folder...").clicked() {
                    let _ = _sender.send(UiMessages::PickFolder(None));
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("❌ Exit").clicked() {
                    std::process::exit(0);
                }
            });

            // ── View 菜单 ──
            ui.menu_button("View", |ui| {
                if ui.button("🔄 Main Window").clicked() {
                    ui.close_menu();
                }
                if ui.button("🔪 Chip Window").clicked() {
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("📋 Task Panel").clicked() {
                    let _ = _sender.send(UiMessages::ToggleTaskPanel);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("⚙ Settings...").clicked() {
                    let _ = _sender.send(UiMessages::ToggleSettings);
                    ui.close_menu();
                }
            });

            // ── Help 菜单 ──
            ui.menu_button("Help", |ui| {
                if ui.button("ℹ About...").clicked() {
                    ui.close_menu();
                }
                if ui.button("🔍 Check FFmpeg Version").clicked() {
                    ui.close_menu();
                }
            });
        });
    });
}
