use std::path::PathBuf;

use crate::app::state::WindowState;
use crate::core::utils::MediaMetadata;

pub enum UiMessages {
    Increment,
    Decrement,
    LoadData(String), // 触发异步数据加载
    DataLoaded(Result<String, String>), // 异步操作的结果
    PickFile(Option<u8>),
    PickFolder(Option<u8>),
    FileSelected(Option<u8>, Option<PathBuf>),
    FolderSelected(Option<u8>, Option<PathBuf>),
    StartFFMPEG,
    StopFFMPEG,

    // ── UI 控制 ──
    /// 切换设置对话框显示/隐藏
    ToggleSettings,
    /// 切换任务面板显示/隐藏
    ToggleTaskPanel,
    /// 切换关于对话框显示/隐藏
    ToggleAbout,
    /// 切换窗口状态（主窗口 / 切片窗口）
    SwitchWindow(WindowState),
    /// 检查 FFmpeg 版本
    CheckFFmpegVersion,

    // ── 播放器事件 ──
    /// 播放事件（开始、停止等）
    PlaybackEvent { event: String, data: Option<String> },
    /// 播放状态更新
    PlaybackStateUpdate { state: crate::core::player::mpv::PlaybackState },

    // ── 元数据 ──
    /// 媒体文件元数据提取完成
    MetadataLoaded(Result<MediaMetadata, String>),

    // ── 任务系统消息 ──
    /// 任务进度更新
    JobProgress { job_id: u64, percent: f32 },
    /// 任务执行完成
    JobComplete { job_id: u64 },
    /// 任务执行失败
    JobError { job_id: u64, error: String },
    /// 通知 TaskManager 尝试派发队列中的下一个任务
    TryDispatchNext,
}
