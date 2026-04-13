pub mod logic;

use eframe::egui;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use rfd::FileDialog;
use std::path::PathBuf;

const COMBO_WIDTH: f32 = 220.0;

// 用于在 Windows 上隐藏命令行窗口
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use crate::logic::{get_encoders, get_formats, get_pixel_formats,
                   EncoderInfo, FormatInfo, PixelFormatInfo};

fn main() -> Result<(), eframe::Error> {
    load_fonts(&Default::default());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 480.0])
            .with_resizable(false)
            .with_maximized(false),
        ..Default::default()
    };
    eframe::run_native(
        "FFmpeg GUI Runner",
        options,
        Box::new(|_cc| Box::new(FfmpegApp::default())),
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
    /// 用户输入的参数字符串
    input_args: String,
    file_path1: Option<std::path::PathBuf>, // 第一个文件选择框的路径
    file_path2: Option<std::path::PathBuf>, // 第二个文件选择框的路径

    file_picker_tx: Option<mpsc::SyncSender<PathBuf>>,
    file_picker_rx: Option<mpsc::Receiver<PathBuf>>,

    // 用于标识哪个文件选择器被触发
    active_picker: Option<u8>, // 1 代表第一个，2 代表第二个

    // --- 中间下拉框 ---
    // 所有可用的描述列表
    encoder_descriptions: Vec<String>,
    format_descriptions: Vec<String>,
    pixel_format_names: Vec<String>,

    // 当前选中的项
    selected_encoder: String,
    selected_format: String,
    selected_pixel_format: String,

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
            input_args: "".to_string(),
            file_path1: None,
            file_path2: None,
            file_picker_tx: None,
            file_picker_rx: None,
            active_picker: None,
            encoder_descriptions: Vec::new(),
            format_descriptions: Vec::new(),
            pixel_format_names: Vec::new(),
            selected_encoder: String::new(),
            selected_format: String::new(),
            selected_pixel_format: String::new(),
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
                        self.output_lines.push(format!(">>> 进程退出，状态码: {}", status));
                        self.child = None;
                    }
                    Ok(None) => {
                        // 仍在运行，无需操作
                    }
                    Err(e) => {
                        self.is_running = false;
                        self.error_message = Some(format!("检查进程状态失败: {}", e));
                        self.child = None;
                    }
                }
            }
        }

        self.check_file_picker();

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
            egui::Frame::default().inner_margin(egui::Margin::same(10.0)).show(ui, |ui| {
                ui.heading("FFmpeg GUI operator");

                // 显示运行状态
                if self.is_running {
                    ui.label("Now running ffmpeg...");
                }

                // 普通输入框，占据一定宽度
                ui.label("Input:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.input_args)
                        .desired_width(700.0)
                );

                ui.add_space(10.0);

                // ---------- 顶部区域：两个文件选择 + 按钮 ----------
                ui.horizontal(|ui| {
                    // 预留一些空间给左侧的输入框和按钮
                    let available_width = ui.available_width();
                    let picker_width = (available_width - 100.0) / 2.0; // 粗略计算，100.0 预留给 Run/Stop 按钮

                    // === 第一个文件选择 ===
                    ui.horizontal(|ui| {
                        ui.set_width(picker_width);
                        // 文件路径显示标签
                        let path1_text = self.file_path1
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("Select the files to be converted");

                        // 使用 add_sized 限制标签的宽度，防止长文件名破坏布局
                        ui.add_sized([picker_width - 70.0, ui.spacing().interact_size.y],
                                     egui::Label::new(path1_text).truncate(true)
                        );

                        // 浏览按钮
                        if ui.button("Browse...").clicked() {
                            self.pick_file(1); // 触发第一个文件选择器
                        }
                    });

                    ui.add_space(10.0);

                    // === 第二个文件选择 ===
                    ui.horizontal(|ui| {
                        ui.set_width(picker_width);
                        let path2_text = self.file_path2
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("Select the exported file");

                        ui.add_sized([picker_width - 70.0, ui.spacing().interact_size.y],
                                     egui::Label::new(path2_text).truncate(true)
                        );

                        if ui.button("Browse...").clicked() {
                            self.pick_file(2); // 触发第二个文件选择器
                        }
                    });

                    // 弹性空间，将运行/停止按钮推到最右边
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button_enabled = !self.input_args.trim().is_empty() && !self.is_running;
                        if ui.add_enabled(button_enabled, egui::Button::new("Run")).clicked() {
                            self.start_ffmpeg();
                        }
                        if self.is_running {
                            if ui.button("Stop").clicked() {
                                self.stop_ffmpeg();
                            }
                        }
                    });
                });

                // 第一条分割线
                ui.separator();

                // ---------- 中间区域 ----------
                ui.label("Encoders:");
                let combo_encoder_response = egui::ComboBox::from_id_source("encoder")
                    .selected_text(truncate_str(&self.selected_encoder, 100))
                    .width(680.0)
                    .show_ui(ui, |ui| {
                        for desc in &self.encoder_descriptions {
                            let display_text = truncate_str(desc, 100);
                            let is_selected = self.selected_encoder == *desc;
                            // 使用 selectable_label 获得 Response
                            let response = ui.selectable_label(is_selected, display_text);
                            // 为当前选项添加悬停提示（显示完整文本）
                            if desc.len() > 100 {  // 只有当文本被截断时才显示完整提示，避免冗余
                                response.clone().on_hover_text(desc);
                            }
                            // 手动处理点击事件
                            if response.clicked() {
                                self.selected_encoder = desc.clone();
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    });
                if !self.selected_encoder.is_empty() && self.selected_encoder != truncate_str(&self.selected_encoder, 100) {
                    combo_encoder_response.response.on_hover_text(&self.selected_encoder);
                }

                ui.add_space(5.0);
                ui.label("Layouts:");
                let combo_format_response = egui::ComboBox::from_id_source("format")
                    .selected_text(truncate_str(&self.selected_format, 100))
                    .width(680.0)
                    .show_ui(ui, |ui| {
                        for desc in &self.format_descriptions {
                            let display_text = truncate_str(desc, 100);
                            let is_selected = self.selected_format == *desc;
                            // 使用 selectable_label 获得 Response
                            let response = ui.selectable_label(is_selected, display_text);
                            // 为当前选项添加悬停提示（显示完整文本）
                            if desc.len() > 100 {  // 只有当文本被截断时才显示完整提示，避免冗余
                                response.clone().on_hover_text(desc);
                            }
                            // 手动处理点击事件
                            if response.clicked() {
                                self.selected_format = desc.clone();
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    });
                if !self.selected_encoder.is_empty() && self.selected_encoder != truncate_str(&self.selected_encoder, 100) {
                    combo_format_response.response.on_hover_text(&self.selected_encoder);
                }

                ui.add_space(5.0);
                ui.label("PixFmts:");
                egui::ComboBox::from_id_source("pixel")
                    .selected_text(&self.selected_pixel_format)
                    .width(680.0)
                    .show_ui(ui, |ui| {
                        for name in &self.pixel_format_names {
                            ui.selectable_value(&mut self.selected_pixel_format, name.clone(), name);
                        }
                    });

                // 第二条分割线
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

                // 显示运行状态
                if self.is_running {
                    ui.label("Now running ffmpeg...");
                }
            });
        });

        // 持续请求重绘，以便及时接收新输出（只要有通道存在或进程在运行）
        if self.is_running || self.receiver.is_some() {
            ctx.request_repaint();
        }
    }
}

impl FfmpegApp {
    /// 触发文件选择对话框
    fn pick_file(&mut self, picker_id: u8) {
        let (tx, rx) = mpsc::sync_channel(1);
        self.file_picker_tx = Some(tx.clone());
        self.file_picker_rx = Some(rx);
        self.active_picker = Some(picker_id);

        // 在新线程中执行阻塞的文件对话框
        std::thread::spawn(move || {
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

    fn load_ffmpeg_data(&mut self) {
        // 获取原始数据
        let encoders = get_encoders();
        let formats = get_formats();
        let pixel_formats = get_pixel_formats();

        // 提取描述
        self.encoder_descriptions = encoders.iter().map(|e| e.description.clone()).collect();
        self.format_descriptions = formats.iter().map(|f| f.description.clone()).collect();
        self.pixel_format_names = pixel_formats.iter().map(|p| p.name.clone()).collect();

        // 设置默认选中第一项（如果有的话），否则为空字符串
        self.selected_encoder = self.encoder_descriptions.first().cloned().unwrap_or_default();
        self.selected_format = self.format_descriptions.first().cloned().unwrap_or_default();
        self.selected_pixel_format = self.pixel_format_names.first().cloned().unwrap_or_default();
    }
    /// 启动 ffmpeg 进程和读取线程
    fn start_ffmpeg(&mut self) {
        // 清空之前的状态
        self.output_lines.clear();
        self.error_message = None;
        self.is_running = true;

        // 解析用户输入的参数（简单按空格分割，支持引号分组）
        let args = match shellwords::split(&self.input_args) {
            Ok(args) => args,
            Err(e) => {
                self.error_message = Some(format!("Parametric parsing failure: {}", e));
                self.is_running = false;
                return;
            }
        };

        if args.is_empty() {
            self.error_message = Some("No parameters are provided".to_string());
            self.is_running = false;
            return;
        }

        // 创建命令
        let mut cmd = Command::new("ffmpeg");
        cmd.args(&args);

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
        self.output_lines.push(">>> Start run ffmpeg ...".to_string());
    }

    /// 停止 ffmpeg 进程
    fn stop_ffmpeg(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill(); // 强制终止
            let _ = child.wait();
            self.output_lines.push(">>> User manual termination ffmpeg".to_string());
        }
        self.is_running = false;
        self.receiver = None;
    }
}

fn load_fonts(ctx: &egui::Context) {
    // 1. 获取默认的字体定义作为起点
    let mut fonts = egui::FontDefinitions::default();

    // 2. 注册新字体数据，并命名为 "consola"
    //    (注意: 字体文件路径可能因你的项目结构而异)
    fonts.font_data.insert(
        "consola".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/consola.ttf")),
    );

    // 3. 将字体应用到指定的字体家族 (FontFamily)
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional) // 获取比例字体家族
        .unwrap()
        .insert(0, "consola".to_owned()); // 插入到最高优先级

    // 4. 应用字体定义到 egui 上下文
    ctx.set_fonts(fonts);
}

// 辅助函数：截断字符串
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        format!("{}…", s.chars().take(max_chars).collect::<String>())
    } else {
        s.to_string()
    }
}

// 简单的参数分割辅助（使用 shellwords crate 会更健壮，这里为了简化手动实现一个基础版）
mod shellwords {
    /// 按空格分割字符串，支持双引号分组
    pub fn split(s: &str) -> Result<Vec<String>, String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        args.push(current.clone());
                        current.clear();
                    }
                }
                '\\' if in_quotes => {
                    // 处理转义引号
                    if let Some(&next) = chars.peek() {
                        if next == '"' {
                            current.push('"');
                            chars.next();
                        } else {
                            current.push('\\');
                        }
                    } else {
                        current.push('\\');
                    }
                }
                _ => current.push(c),
            }
        }
        if !current.is_empty() {
            args.push(current);
        }
        Ok(args)
    }
}