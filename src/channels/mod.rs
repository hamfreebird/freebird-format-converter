use egui_inbox::UiInboxSender;
use crate::app::state::FfmpegApp;
use crate::channels::messages::UiMessages;
use crate::core;

pub mod messages;

pub fn process_message(state: &mut FfmpegApp, sender: UiInboxSender<UiMessages>, msg: UiMessages) {
    match msg {
        UiMessages::PickFile(id) => {
            core::utils::scanner::pick_file(
                &mut state.file_picker_tx,
                &mut state.file_picker_rx,
                &mut state.active_picker,
                1
            );
        },
        UiMessages::PickFolder(id) => {
            core::utils::scanner::pick_folder(
                &mut state.folder_picker_tx,
                &mut state.folder_picker_rx,
                &mut state.active_folder_picker,
                1
            );
        },
        UiMessages::StartFFMPEG => {
            core::processor::ffmpeg::start_ffmpeg(state);
        },
        UiMessages::StopFFMPEG => {
            core::processor::ffmpeg::stop_ffmpeg(state);
        },
        _ => {}
    }
}