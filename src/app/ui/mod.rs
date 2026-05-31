use egui::Ui;
use crate::app::state::{FfmpegApp, WindowState};

pub mod dialogs;
pub mod media_library;
pub mod menu;
pub mod preview;
pub mod setting;
pub mod task_panel;
pub mod main_window;
pub mod chip;

pub fn render(ui: &mut Ui,
              state: &mut FfmpegApp) {
    let ctx = ui.ctx().clone();

    // ── 顶部菜单栏 ──
    let sender = state.inbox.sender();
    menu::render_menu_bar(&ctx, &sender);

    // ── 主页面内容 ──
    if state.window_state == WindowState::MainWindow {
        main_window::render_main_window(
            ui,
            Option::from(1u8),
            &state.file_path1,
            &state.folder_path1,
            state.inbox.sender(),
            state.is_running,
            state.encoder_info.clone(),
            state.format_info.clone(),
            state.pixel_format_info.clone(),
            state.encoder_name.clone(),
            state.format_name.clone(),
            state.pixel_format_names.clone(),
            &mut state.selected_encoder,
            &mut state.selected_format,
            &mut state.selected_pixel_format,
            &mut state._is_video,
            &mut state._is_audio,
            &mut state._is_subtitle,
            state.error_message.clone(),
            &mut state.bitrate,
            &mut state.constant_rate_factor,
            &mut state.coding_default,
            &mut state.gop,
            &mut state.output_lines,
            &state.metadata,
        )
    } else if state.window_state == WindowState::ChipWindow {
        chip::render_chip_window(
            ui,
            state.error_message.clone(),
        )
    }

    // ── 浮动窗口 ──
    setting::render_settings_dialog(
        &ctx,
        &mut state.settings,
        &mut state.show_settings,
    );

    // 任务面板
    if let Some(ref tm) = state.task_manager {
        let jobs = tm.all_jobs().clone();
        task_panel::render_task_panel(
            &ctx,
            &mut state.show_task_panel,
            &jobs,
            |job_id| {
                // cancel callback - 发送到 inbox 处理
                let _ = state.inbox.sender().send(
                    crate::channels::messages::UiMessages::JobComplete { job_id }
                );
            },
            || {
                // clear callback
                if let Some(ref mut tm) = state.task_manager {
                    tm.clear_completed();
                }
            },
        );
    }

    // 关于对话框
    dialogs::show_about_dialog(
        &ctx,
        &mut state.show_about,
    );
}
