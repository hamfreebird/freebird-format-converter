use std::collections::VecDeque;
use egui::Ui;
use crate::app::state::FfmpegApp;
use crate::app::ui;
use crate::channels::messages::UiMessages;

mod dialogs;
mod media_library;
mod menu;
mod preview;
mod setting;
mod task_panel;
pub mod main_window;

pub fn render(ui: &mut Ui,
              state: &mut FfmpegApp) {
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
        &mut state.bitrate,                  // 目标比特率
        &mut state.constant_rate_factor,     // 恒定质量模式 0-51
        &mut state.coding_default,       // 编码预设
        &mut state.gop,
        &mut state.output_lines,
    )
}