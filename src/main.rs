pub mod logic;

use eframe::egui;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use image::GenericImageView;

// 用于在 Windows 上隐藏命令行窗口
use crate::logic::{
    EncoderInfo, FormatInfo, PixelFormatInfo, get_encoders, get_formats, get_pixel_formats,
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use crate::logic::dependent::ensure_ffmpeg;
use crate::logic::execute::{build_ffmpeg_command, validate_transcode_params};

fn main() -> Result<(), eframe::Error> {
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
        Box::new(|_cc| Box::new(FfmpegApp::new(_cc))),
    )
}

struct FfmpegApp {
    /// 是否正在运行 ffmpeg
    is_running: bool,
    /// 子进程句柄（用于检查退出状态和终止）
    child: Option<std::process::Child>,
    /// 接收来自读取线程的输出行的通道
    receiver: Option<mpsc::Receiver<String>>,
    /// 错误信息（例如启动失败）
    error_message: Option<String>,

    // --- 顶部输入区 ---
    file_path1: Option<PathBuf>, // 第一个文件选择框的路径
    file_path2: Option<PathBuf>, // 第二个文件选择框的路径

    file_picker_tx: Option<mpsc::SyncSender<PathBuf>>,
    file_picker_rx: Option<mpsc::Receiver<PathBuf>>,

    // 用于标识哪个文件选择器被触发
    active_picker: Option<u8>, // 1 代表第一个，2 代表第二个

    // 文件夹选择相关字段
    folder_picker_tx: Option<mpsc::SyncSender<PathBuf>>,
    folder_picker_rx: Option<mpsc::Receiver<PathBuf>>,
    active_folder_picker: Option<u8>,
    folder_path1: Option<PathBuf>,
    folder_path2: Option<PathBuf>,

    // --- 中间下拉框 ---
    // 所有信息
    encoder_info: Vec<EncoderInfo>,
    format_info: Vec<FormatInfo>,
    pixel_format_info: Vec<PixelFormatInfo>,

    // 所有可用的列表
    encoder_name: Vec<String>,
    format_name: Vec<String>,
    pixel_format_names: Vec<String>,

    // 当前选中的项
    selected_encoder: String,
    selected_format: String,
    selected_pixel_format: String,

    // --- 详细参数 ---
    bitrate: String,                  // 目标比特率
    constant_rate_factor: String,     // 恒定质量模式 0-51
    coding_default: String,       // 编码预设
    gop: String,                      // GOP关键帧间隔
    _is_video: bool,
    _is_audio: bool,
    _is_subtitle: bool,

    // --- 底部输出框 ---
    /// 累积的输出日志（每行一条）
    output_lines: Vec<String>,
}

impl Default for FfmpegApp {
    fn default() -> Self {
        let mut app = Self {
            // ... 初始化其他字段 ...
            is_running: false,
            child: None,
            receiver: None,
            error_message: None,
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
            encoder_info: vec![],
            format_info: vec![],
            pixel_format_info: vec![],
            encoder_name: Vec::new(),
            format_name: Vec::new(),
            pixel_format_names: Vec::new(),
            selected_encoder: String::new(),
            selected_format: String::new(),
            selected_pixel_format: String::new(),
            bitrate: 2000.to_string(),
            constant_rate_factor: 21.to_string(),
            coding_default: "".to_string(),
            gop: 120.to_string(),
            _is_video: false,
            _is_audio: false,
            _is_subtitle: false,
            output_lines: vec![],
        };
        // 注意：此处直接调用会阻塞 GUI 启动，最好在另一个线程加载
        // 简单演示可在此调用，但正式应用推荐异步加载
        app.load_ffmpeg_data();
        app
    }
}

impl eframe::App for FfmpegApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. 检查子进程是否结束（如果正在运行）
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

        // 2. 从通道接收新的输出行（无论是否正在运行，确保清空缓冲区）
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

        // 3. 绘制 UI
        egui::CentralPanel::default().show(ctx, |ui| {
            // 设置整体边距，让内容不那么挤
            egui::Frame::default()
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Log: ");
                        // 显示运行状态
                        ui.label(self.error_message
                            .as_deref()                     // Option<&str>
                            .unwrap_or("As right as rain~  ヾ(≧▽≦*)o")           // &str
                            .to_string());

                    });

                    ui.separator();

                    // ---------- 顶部区域：两个文件选择 + 按钮 ----------
                    ui.horizontal(|ui| {
                        // 预留一些空间给左侧的输入框和按钮
                        let available_width = ui.available_width();
                        let picker_width = (available_width - 100.0) / 2.0; // 粗略计算，100.0 预留给 Run/Stop 按钮

                        // === 第一个文件选择 ===
                        ui.horizontal(|ui| {
                            ui.set_width(picker_width);
                            // 文件路径显示标签
                            let path1_text = self
                                .file_path1
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("Select the files to be converted");

                            // 使用 add_sized 限制标签的宽度，防止长文件名破坏布局
                            ui.add_sized(
                                [picker_width - 70.0, ui.spacing().interact_size.y],
                                egui::Label::new(path1_text).truncate(true),
                            );

                            // 浏览按钮
                            if ui.button("Browse...").clicked() {
                                self.pick_file(1); // 触发第一个文件选择器
                            }
                        });

                        ui.add_space(10.0);

                        // === 第一个文件夹选择 ===
                        ui.horizontal(|ui| {
                            ui.set_width(picker_width);
                            let path2_text = self
                                .folder_path1
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .unwrap_or("Select the folder as the output directory");

                            ui.add_sized(
                                [picker_width - 70.0, ui.spacing().interact_size.y],
                                egui::Label::new(path2_text).truncate(true),
                            );

                            if ui.button("Browse...").clicked() {
                                self.pick_folder(1); // 触发第一个文件夹选择器
                            }
                        });

                        // 弹性空间，将运行/停止按钮推到最右边
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let button_enabled = !self.is_running;
                            if ui
                                .add_enabled(button_enabled, egui::Button::new("Run"))
                                .clicked()
                            {
                                self.start_ffmpeg();
                            }
                            if self.is_running {
                                if ui.button("Stop").clicked() {
                                    self.stop_ffmpeg();
                                }
                            }
                        });
                    });

                    ui.separator();

                    // ---------- 中间区域 ----------
                    ui.horizontal(|ui| {
                        ui.label("Encoders:");
                        let combo_encoder_response = egui::ComboBox::from_id_source("encoder")
                            .selected_text(truncate_str(&self.selected_encoder, 100))
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for (index, name) in self.encoder_name.iter().enumerate() {
                                    let display_text = truncate_str(name, 100);
                                    let is_selected = self.selected_encoder == *name;
                                    // 使用 selectable_label 获得 Response
                                    let response = ui.selectable_label(is_selected, display_text);
                                    // 始终显示完整提示，显示对应的description
                                    if let Some(encoder_info) = self.encoder_info.get(index) {
                                        let mut type_info: String = "Unknown".to_string();
                                        if (encoder_info.is_video == true) & (encoder_info.is_audio == true)
                                            {type_info = "Video/Audio".to_string()}
                                        else if (encoder_info.is_video == true) & (encoder_info.is_audio == false)
                                            {type_info = "Video".to_string()}
                                        else if (encoder_info.is_video == false) & (encoder_info.is_audio == true)
                                            {type_info = "Audio".to_string()};
                                        if encoder_info.is_subtitle == true {type_info = "Subtitle".to_string()};
                                        let hover_text = format!(
                                            "Type: {}\nDescription: {}\nFrame-level multithreading: {}\n\
                                            Slice-level multithreading: {}\nCodec is experimental: {}\n",
                                            type_info,
                                            &encoder_info.description,
                                            &encoder_info.is_frame_multithreading,
                                            &encoder_info.is_slice_multithreading,
                                            &encoder_info.is_experimental,
                                        );
                                        response.clone().on_hover_text(hover_text);
                                    }
                                    // 手动处理点击事件
                                    if response.clicked() {
                                        self.selected_encoder = name.clone();
                                        self._is_video = self.encoder_info.get(index).unwrap().is_video;
                                        self._is_audio = self.encoder_info.get(index).unwrap().is_audio;
                                        self._is_subtitle = self.encoder_info.get(index).unwrap().is_subtitle;
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                }
                            });
                        // 为当前选中的编码器添加悬停提示，显示对应的description
                        if !self.selected_encoder.is_empty() {
                            if let Some(encoder_info) = self
                                .encoder_info
                                .iter()
                                .find(|e| e.name == self.selected_encoder)
                            {
                                let mut type_info: String = "Unknown".to_string();
                                if (encoder_info.is_video == true) & (encoder_info.is_audio == true)
                                {type_info = "Video/Audio".to_string()}
                                else if (encoder_info.is_video == true) & (encoder_info.is_audio == false)
                                {type_info = "Video".to_string()}
                                else if (encoder_info.is_video == false) & (encoder_info.is_audio == true)
                                {type_info = "Audio".to_string()};
                                if encoder_info.is_subtitle == true {type_info = "Subtitle".to_string()};
                                let hover_text = format!(
                                    "Type: {}\nDescription: {}\nFrame-level multithreading: {}\n\
                                    Slice-level multithreading: {}\nCodec is experimental: {}\n",
                                    type_info,
                                    &encoder_info.description,
                                    &encoder_info.is_frame_multithreading,
                                    &encoder_info.is_slice_multithreading,
                                    &encoder_info.is_experimental,
                                );
                                combo_encoder_response.response.on_hover_text(hover_text);
                            } else if self.selected_encoder
                                != truncate_str(&self.selected_encoder, 100)
                            {
                                combo_encoder_response.response.on_hover_text(&self.selected_encoder);
                            }
                        }

                        ui.add_space(5.0);
                        ui.label("Layouts:");
                        let combo_format_response = egui::ComboBox::from_id_source("format")
                            .selected_text(truncate_str(&self.selected_format, 100))
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for (index, name) in self.format_name.iter().enumerate() {
                                    let display_text = truncate_str(name, 100);
                                    let is_selected = self.selected_format == *name;
                                    // 使用 selectable_label 获得 Response
                                    let response = ui.selectable_label(is_selected, display_text);
                                    // 始终显示完整提示，显示对应的description
                                    if let Some(format_info) = self.format_info.get(index) {
                                        let hover_text = format!(
                                            "Description: {}\nCan be read/unsealed as input: {}\n\
                                            Can be written/encapsulated as output: {}",
                                            &format_info.description,
                                            &format_info.can_mux,
                                            &format_info.can_demux
                                        );
                                        response.clone().on_hover_text(hover_text);
                                    }
                                    // 手动处理点击事件
                                    if response.clicked() {
                                        self.selected_format = name.clone();
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                }
                            });
                        // 为当前选中的格式添加悬停提示，显示对应的description
                        if !self.selected_format.is_empty() {
                            if let Some(format_info) = self
                                .format_info
                                .iter()
                                .find(|f| f.name == self.selected_format)
                            {
                                let hover_text = format!(
                                    "Description: {}\nCan be read/unsealed as input: {}\n\
                                    Can be written/encapsulated as output: {}",
                                    &format_info.description,
                                    &format_info.can_mux,
                                    &format_info.can_demux
                                );
                                combo_format_response.response.on_hover_text(hover_text);
                            } else if self.selected_format
                                != truncate_str(&self.selected_format, 100)
                            {
                                combo_format_response.response.on_hover_text(&self.selected_format);
                            }
                        }

                        ui.add_space(5.0);
                        ui.label("PixFmts:");
                        let combo_pixel_response = egui::ComboBox::from_id_source("pixel")
                            .selected_text(&self.selected_pixel_format)
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for (index, name) in self.pixel_format_names.iter().enumerate() {
                                    let response = ui.selectable_value(
                                        &mut self.selected_pixel_format,
                                        name.clone(),
                                        name,
                                    );
                                    // 始终显示完整提示，显示详细信息
                                    if let Some(pixel_info) = self.pixel_format_info.get(index) {
                                        let hover_text = format!(
                                            "Name: {}\nInput: {}\nOutput: {}\nNumber per pixel: {}",
                                            pixel_info.name,
                                            if pixel_info.input_ok { "True" } else { "False" },
                                            if pixel_info.output_ok { "True" } else { "False" },
                                            pixel_info.bits_per_pixel
                                        );
                                        response.on_hover_text(hover_text);
                                    }
                                }
                            });
                        // 为当前选中的像素格式添加悬停提示
                        if !self.selected_pixel_format.is_empty() {
                            if let Some(pixel_info) = self
                                .pixel_format_info
                                .iter()
                                .find(|p| p.name == self.selected_pixel_format)
                            {
                                let hover_text = format!(
                                    "Name: {}\nInput: {}\nOutput: {}\nNumber per pixel: {}",
                                    pixel_info.name,
                                    if pixel_info.input_ok { "True" } else { "False" },
                                    if pixel_info.output_ok { "True" } else { "False" },
                                    pixel_info.bits_per_pixel
                                );
                                combo_pixel_response.response.on_hover_text(hover_text);
                            }
                        }
                    });

                    ui.separator();

                    // 输入详细参数
                    ui.horizontal(|ui| {
                        ui.label("Bitrate:");
                        ui.add(egui::TextEdit::singleline(&mut self.bitrate).desired_width(50.0));
                        ui.add_space(10.0);
                        ui.label("crf:");
                        ui.add(egui::TextEdit::singleline(&mut self.constant_rate_factor).desired_width(50.0));
                        ui.add_space(10.0);
                        ui.label("Preset:");
                        ui.add(egui::TextEdit::singleline(&mut self.coding_default).desired_width(50.0));
                        ui.add_space(10.0);
                        ui.label("GOP:");
                        ui.add(egui::TextEdit::singleline(&mut self.gop).desired_width(50.0));
                        ui.add_space(10.0);
                    });

                    ui.separator();

                    // ---------- 底部区域：多行文本输出框 ----------
                    // 输出显示区域
                    ui.label("Output:");
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.output_lines.join("\n"))
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(20)
                                    .interactive(false) // 只读
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                });
        });

        // 持续请求重绘，以便及时接收新输出（只要有通道存在或进程在运行）
        if self.is_running || self.receiver.is_some() {
            ctx.request_repaint();
        }
    }
}

impl FfmpegApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_fonts(&cc.egui_ctx);
        Self::default()
    }

    /// 触发文件选择对话框
    fn pick_file(&mut self, picker_id: u8) {
        let (tx, rx) = mpsc::sync_channel(1);
        self.file_picker_tx = Some(tx.clone());
        self.file_picker_rx = Some(rx);
        self.active_picker = Some(picker_id);

        // 在新线程中执行阻塞的文件对话框
        thread::spawn(move || {
            // 在这里调用 rfd 的阻塞API[reference:1]
            let file = rfd::FileDialog::new().pick_file();

            // 如果用户选择了文件，则通过通道发送路径
            if let Some(path) = file {
                // 注意：这里可以不用处理发送失败的情况
                let _ = tx.send(path);
            }
        });
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

    /// 触发文件夹选择对话框
    pub fn pick_folder(&mut self, picker_id: u8) {
        let (tx, rx) = mpsc::sync_channel(1);
        self.folder_picker_tx = Some(tx.clone());
        self.folder_picker_rx = Some(rx);
        self.active_folder_picker = Some(picker_id);

        // 在新线程中执行阻塞的文件夹对话框
        thread::spawn(move || {
            // 调用 rfd 的 pick_folder 方法选择文件夹
            let folder = rfd::FileDialog::new().pick_folder();

            // 如果用户选择了文件夹，则通过通道发送路径
            if let Some(path) = folder {
                let _ = tx.send(path);
            }
        });
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

        // 提取名称
        self.encoder_name = self.encoder_info.iter().map(|e| e.name.clone()).collect();
        self.format_name = self.format_info.iter().map(|f| f.name.clone()).collect();
        self.pixel_format_names = self
            .pixel_format_info
            .iter()
            .map(|p| p.name.clone())
            .collect();

        // 设置默认选中第一项（如果有的话），否则为空字符串
        self.selected_encoder = self.encoder_name.first().cloned().unwrap_or_default();
        self.selected_format = self.format_name.first().cloned().unwrap_or_default();
        self.selected_pixel_format = self.pixel_format_names.first().cloned().unwrap_or_default();
    }
    /// 启动 ffmpeg 进程和读取线程
    fn start_ffmpeg(&mut self) {
        // 清空之前的状态
        self.output_lines.clear();
        self.error_message = None;
        self.is_running = true;

        // 解析用户输入的参数（简单按空格分割，支持引号分组）
        // let args = match shellwords::split(&self.input_args) {
        //     Ok(args) => args,
        //     Err(e) => {
        //         self.error_message = Some(format!("Parametric parsing failure: {}", e));
        //         self.is_running = false;
        //         return;
        //     }
        // };

        // if args.is_empty() {
        //     self.error_message = Some("No parameters are provided".to_string());
        //     self.is_running = false;
        //     return;
        // }

        if let Err(e) = validate_transcode_params(
            &self.selected_encoder,
            self._is_video,
            self._is_audio,
            self._is_subtitle,
            &self.selected_format,
            &self.selected_pixel_format,
            &self.bitrate,
            &self.constant_rate_factor,
            &self.gop,
            self.file_path1.as_deref(),
            self.folder_path1.as_deref(),
        ) {
            self.error_message = format!("Invalid parameter: {}", e).into();
            eprintln!("Invalid parameter: {}", e);
            // 根据错误类型决定是否继续或返回
            return;
        }

        let build_result = build_ffmpeg_command(
            &self.selected_encoder,          // encoder: &str
            self._is_video,          // bool
            self._is_audio,          // bool
            self._is_subtitle,       // bool
            &self.selected_format,        // container: &str
            &self.selected_pixel_format,          // pix_fmt: &str
            &self.bitrate,          // bitrate: &str
            &self.constant_rate_factor,          // quality: &str
            &self.coding_default,           // preset: &str
            &self.gop,              // gop: &str
            self.file_path1.as_deref(),
            self.folder_path1.as_deref(),
        );

        // 创建命令
        let mut cmd: Command = match build_result {
            Ok(c) => c,
            Err(e) => {
                self.error_message = Some(format!("参数错误: {}", e));
                self.is_running = false;
                return;
            }
        };

        // 隐藏 Windows 控制台窗口
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        // 捕获 stdout 和 stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // 启动子进程
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.error_message = Some(format!("Failed run ffmpeg: {}", e));
                self.is_running = false;
                return;
            }
        };

        // 获取输出句柄
        let stdout = child.stdout.take().expect("Unacquired stdout");
        let stderr = child.stderr.take().expect("Unacquired stderr");

        // 创建通道用于将输出发送到 GUI 线程
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);

        // 启动读取线程（分别读取 stdout 和 stderr）
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx_clone.send(format!("[stdout] {}", l)).is_err() {
                            break; // 接收端已关闭
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(format!("[stderr] {}", l)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.child = Some(child);
        self.output_lines
            .push(">>> Start run ffmpeg ...".to_string());
    }

    /// 停止 ffmpeg 进程
    fn stop_ffmpeg(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill(); // 强制终止
            let _ = child.wait();
            self.output_lines
                .push(">>> User manual termination ffmpeg".to_string());
        }
        self.is_running = false;
        self.receiver = None;
    }
}

fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 注册多个字体数据
    fonts.font_data.insert(
        "Ubuntu-Light".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Ubuntu-Light.ttf")),
    );
    fonts.font_data.insert(
        "simhei".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/simhei.ttf")),
    );

    // 为 Proportional 家族设置优先级顺序
    let proportional_fonts = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional_fonts.clear();               // 清除默认字体
    proportional_fonts.push("Ubuntu-Light".to_owned());
    proportional_fonts.push("simhei".to_owned());

    // 为 Monospace 家族单独设置
    let monospace_fonts = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    monospace_fonts.clear();
    monospace_fonts.push("Ubuntu-Light".to_owned());
    monospace_fonts.push("simhei".to_owned());

    ctx.set_fonts(fonts);
}

fn load_icon_data() -> egui::IconData {
    // 将图像文件（如 favicon.png）作为字节数组嵌入
    let image_bytes = include_bytes!("../assets/freebird-format-converter.ico");
    // 使用 image 库解码图像
    let image = image::load_from_memory(image_bytes).expect("Failed to load icon");
    // 确保图像尺寸合适并转换为 RGBA
    let image = image.into_rgba8();

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    egui::IconData {
        rgba,
        width,
        height,
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
