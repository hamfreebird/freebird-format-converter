use std::collections::VecDeque;
use egui::Ui;
use crate::channels::messages::UiMessages;

mod dialogs;
mod media_library;
mod menu;
mod preview;
mod setting;
mod task_panel;
pub mod main_window;

pub fn render(ui: &mut Ui, message_queue: &mut VecDeque<UiMessages>) {

}