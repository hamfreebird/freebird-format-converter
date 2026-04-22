use std::path::PathBuf;
use std::sync::mpsc;
use egui_inbox::UiInbox;
use crate::channels::messages::UiMessages;
use crate::core::{ColorInfo, EncoderInfo, FormatInfo, PixelFormatInfo};

pub struct FfmpegApp {
    /// 是否正在运行 ffmpeg
    pub(crate) is_running: bool,
    /// 子进程句柄（用于检查退出状态和终止）
    pub(crate) child: Option<std::process::Child>,
    /// 接收来自读取线程的输出行的通道
    pub(crate) receiver: Option<mpsc::Receiver<String>>,
    /// 错误信息（例如启动失败）
    pub(crate) error_message: Option<String>,

    // --- 页面管理 ---
    pub(crate) window_state: WindowState,

    // --- 顶部输入区 ---
    pub(crate) file_path1: Option<PathBuf>, // 第一个文件选择框的路径
    pub(crate) file_path2: Option<PathBuf>, // 第二个文件选择框的路径

    pub(crate) file_picker_tx: Option<mpsc::SyncSender<PathBuf>>,
    pub(crate) file_picker_rx: Option<mpsc::Receiver<PathBuf>>,

    // 用于标识哪个文件选择器被触发
    pub(crate) active_picker: Option<u8>, // 1 代表第一个，2 代表第二个

    // 文件夹选择相关字段
    pub(crate) folder_picker_tx: Option<mpsc::SyncSender<PathBuf>>,
    pub(crate) folder_picker_rx: Option<mpsc::Receiver<PathBuf>>,
    pub(crate) active_folder_picker: Option<u8>,
    pub(crate) folder_path1: Option<PathBuf>,
    pub(crate) folder_path2: Option<PathBuf>,

    // --- 从 ffmpeg 读取的支持信息 ---
    // 所有信息
    pub(crate) encoder_info: Vec<EncoderInfo>,
    pub(crate) format_info: Vec<FormatInfo>,
    pub(crate) pixel_format_info: Vec<PixelFormatInfo>,
    pub(crate) color_info: Vec<ColorInfo>,

    // 名称
    pub(crate) encoder_name: Vec<String>,
    pub(crate) format_name: Vec<String>,
    pub(crate) pixel_format_names: Vec<String>,
    pub(crate) color_names: Vec<String>,

    // 当前选中的项
    pub(crate) selected_encoder: String,
    pub(crate) selected_format: String,
    pub(crate) selected_pixel_format: String,
    pub(crate) selected_color: String,

    // --- 详细参数 ---
    pub(crate) bitrate: String,                  // 目标比特率
    pub(crate) constant_rate_factor: String,     // 恒定质量模式 0-51
    pub(crate) coding_default: String,       // 编码预设
    pub(crate) gop: String,                      // GOP关键帧间隔
    pub(crate) _is_video: bool,
    pub(crate) _is_audio: bool,
    pub(crate) _is_subtitle: bool,

    // --- 底部输出框 ---
    pub(crate) output_lines: Vec<String>,

    // --- egui_inbox ---
    pub(crate) inbox: UiInbox<UiMessages>,
}

// 页面管理
#[derive(PartialEq)]
pub(crate) enum WindowState {
    MainWindow,
    ChipWindow,
}