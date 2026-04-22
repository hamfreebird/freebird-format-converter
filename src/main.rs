#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod core;
pub mod services;
pub mod channels;

use self::app::state::{FfmpegApp, WindowState};
use self::app::ui;
use self::core::utils::dependent::ensure_ffmpeg;
use crate::channels::process_message;
use crate::core::processor::ffmpeg::{get_colors, get_encoders, get_formats, get_pixel_formats};
use crate::core::utils::config::{load_fonts, load_icon_data};
use eframe::{egui, Frame};
use egui::Ui;
use egui_inbox::UiInbox;
use std::sync::mpsc::TryRecvError;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    ensure_ffmpeg().expect("Opus, something went wrong! The program will continue to run but it may go wrong.");
    let _icon_data = load_icon_data();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 480.0])
            .with_resizable(false)
            .with_maximized(false)
            .with_icon(_icon_data),
        ..Default::default()
    };
    eframe::run_native(
        "freebird format converter v0.1",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(FfmpegApp::new(cc)))
        }),
    )
}

impl Default for FfmpegApp {
    fn default() -> Self {
        let mut app = Self {
            is_running: false,
            child: None,
            receiver: None,
            error_message: None,
            window_state: WindowState::MainWindow,
            file_path1: None,
            file_path2: None,
            file_picker_tx: None,
            file_picker_rx: None,
            active_picker: None,
            folder_picker_tx: None,
            folder_picker_rx: None,
            active_folder_picker: None,
            folder_path1: None,
            folder_path2: None,
            encoder_info: Vec::new(),
            format_info: Vec::new(),
            pixel_format_info: Vec::new(),
            color_info: Vec::new(),
            encoder_name: Vec::new(),
            format_name: Vec::new(),
            pixel_format_names: Vec::new(),
            color_names: Vec::new(),
            selected_encoder: String::new(),
            selected_format: String::new(),
            selected_pixel_format: String::new(),
            selected_color: String::new(),
            bitrate: 2000.to_string(),
            constant_rate_factor: 21.to_string(),
            coding_default: String::new(),
            gop: 120.to_string(),
            _is_video: false,
            _is_audio: false,
            _is_subtitle: false,
            output_lines: Vec::new(),
            inbox: UiInbox::new(),
        };
        // 阻塞 GUI 启动,加载 ffmpeg
        app.load_ffmpeg_data();
        app
    }
}

impl eframe::App for FfmpegApp {
    fn logic(&mut self, _ui: &egui::Context, _frame: &mut Frame) {
        let messages = self.inbox.read_without_ctx();
        for msg in messages {
            process_message(self, msg)
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        // 绘制 UI
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui::render(ui, self);
        });

        // 持续请求重绘
        if self.is_running || self.receiver.is_some() {
            ui.request_repaint();
        }
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut Frame) {
        // 检查子进程是否结束（如果正在运行）
        if self.is_running {
            if let Some(ref mut child) = self.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // 子进程已退出
                        self.is_running = false;
                        self.output_lines
                            .push(format!(">>> Process exit, status code: {}", status));
                        self.child = None;
                    }
                    Ok(None) => {
                        // 仍在运行，无需操作
                    }
                    Err(e) => {
                        self.is_running = false;
                        self.error_message = Some(format!("Check the process state failure: {}", e));
                        self.child = None;
                    }
                }
            }
        }

        // 查看选择的文件/文件夹内容
        self.check_file_picker();
        self.check_folder_picker();

        // 从通道接收新的输出行（无论是否正在运行，确保清空缓冲区）
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(line) => self.output_lines.push(line),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        // 发送端已断开，读取线程结束
                        self.receiver = None;
                        break;
                    }
                }
            }
        }
    }
}

impl FfmpegApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_fonts(&cc.egui_ctx);
        Self::default()
    }

    /// 检查文件选择的结果，应在每一帧的 `update` 函数中调用
    fn check_file_picker(&mut self) {
        if let Some(rx) = &self.file_picker_rx {
            // 使用 try_recv 非阻塞地检查是否有结果[reference:2]
            if let Ok(path) = rx.try_recv() {
                // 根据 active_picker 将路径赋值给正确的字段
                if let Some(id) = self.active_picker.take() {
                    if id == 1 {
                        self.file_path1 = Some(path);
                    } else if id == 2 {
                        self.file_path2 = Some(path);
                    }
                }
                // 清理通道资源
                self.file_picker_tx = None;
                self.file_picker_rx = None;
            }
        }
    }

    /// 检查文件夹选择的结果，应在每一帧的 `update` 函数中调用
    pub fn check_folder_picker(&mut self) {
        if let Some(rx) = &self.folder_picker_rx {
            // 非阻塞检查是否有结果
            if let Ok(path) = rx.try_recv() {
                // 根据 active_picker 将路径赋值给对应字段
                if let Some(id) = self.active_folder_picker.take() {
                    match id {
                        1 => self.folder_path1 = Some(path),
                        2 => self.folder_path2 = Some(path),
                        _ => {}
                    }
                }
                // 清理通道资源
                self.folder_picker_tx = None;
                self.folder_picker_rx = None;
            }
        }
    }

    fn load_ffmpeg_data(&mut self) {
        // 获取原始数据
        self.encoder_info = get_encoders();
        self.format_info = get_formats();
        self.pixel_format_info = get_pixel_formats();
        self.color_info = get_colors();

        // 提取名称
        self.encoder_name = self.encoder_info.iter().map(|e| e.name.clone()).collect();
        self.format_name = self.format_info.iter().map(|f| f.name.clone()).collect();
        self.pixel_format_names = self.pixel_format_info.iter().map(|p| p.name.clone()).collect();
        self.color_names = self.color_info.iter().map(|p| p.name.clone()).collect();

        // 设置默认选中第一项（如果有的话），否则为空字符串
        self.selected_encoder = self.encoder_name.first().cloned().unwrap_or_default();
        self.selected_format = self.format_name.first().cloned().unwrap_or_default();
        self.selected_pixel_format = self.pixel_format_names.first().cloned().unwrap_or_default();
        self.selected_color = self.color_names.first().cloned().unwrap_or_default();
    }
}

// 辅助函数：截断字符串
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        format!("{}…", s.chars().take(max_chars).collect::<String>())
    } else {
        s.to_string()
    }
}