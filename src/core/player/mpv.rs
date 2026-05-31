use serde::{Deserialize, Serialize};
use serde_json::Value;

// ──────────────────────────────────────────────
// MPV IPC 命令
// ──────────────────────────────────────────────

/// MPV IPC 命令
#[derive(Debug, Clone, Serialize)]
pub struct MpvCommand {
    /// 命令名：如 "loadfile", "set_property", "observe_property"
    pub command: Vec<Value>,
}

impl MpvCommand {
    /// 创建基础命令
    fn new(cmd_name: &str) -> Self {
        Self {
            command: vec![Value::String(cmd_name.to_string())],
        }
    }

    /// 带参数的通用命令
    pub fn new_with_args(cmd_name: &str, args: Vec<Value>) -> Self {
        let mut cmd = Self::new(cmd_name);
        cmd.command.extend(args);
        cmd
    }

    // ── 播放控制 ──

    /// 加载文件: loadfile <path> <mode>
    /// mode: "replace" 替换当前播放列表, "append" 追加
    pub fn loadfile(path: &str, mode: &str) -> Self {
        Self::new_with_args(
            "loadfile",
            vec![Value::String(path.to_string()), Value::String(mode.to_string())],
        )
    }

    /// 播放/恢复
    pub fn play() -> Self {
        Self::new_with_args("set_property", vec![
            Value::String("pause".to_string()),
            Value::Bool(false),
        ])
    }

    /// 暂停
    pub fn pause() -> Self {
        Self::new_with_args("set_property", vec![
            Value::String("pause".to_string()),
            Value::Bool(true),
        ])
    }

    /// 切换播放/暂停
    pub fn cycle_pause() -> Self {
        Self::new_with_args("cycle", vec![Value::String("pause".to_string())])
    }

    /// 停止
    pub fn stop() -> Self {
        Self::new("stop")
    }

    /// 退出 MPV
    pub fn quit() -> Self {
        Self::new("quit")
    }

    /// 跳转到指定位置（秒）
    pub fn seek(target_secs: f64, mode: &str) -> Self {
        // mode: "relative", "absolute", "absolute-percent"
        Self::new_with_args(
            "seek",
            vec![
                Value::Number(serde_json::Number::from_f64(target_secs).unwrap()),
                Value::String(mode.to_string()),
            ],
        )
    }

    /// 快进/快退（秒）
    pub fn seek_relative(delta_secs: f64) -> Self {
        Self::seek(delta_secs, "relative")
    }

    /// 跳转到绝对位置
    pub fn seek_absolute(target_secs: f64) -> Self {
        Self::seek(target_secs, "absolute")
    }

    /// 截图
    /// mode: "video" (仅视频), "window" (含 OSD), "subtitles" (含字幕)
    pub fn screenshot(mode: &str) -> Self {
        Self::new_with_args("screenshot", vec![Value::String(mode.to_string())])
    }

    /// 截图到指定文件
    pub fn screenshot_to_file(path: &str) -> Self {
        Self::new_with_args("screenshot-to-file", vec![Value::String(path.to_string())])
    }

    // ── 属性设置 ──

    /// 设置音量 (0-100)
    pub fn set_volume(volume: f64) -> Self {
        Self::new_with_args("set_property", vec![
            Value::String("volume".to_string()),
            Value::Number(serde_json::Number::from_f64(volume.clamp(0.0, 100.0)).unwrap()),
        ])
    }

    /// 设置静音
    pub fn set_mute(mute: bool) -> Self {
        Self::new_with_args("set_property", vec![
            Value::String("mute".to_string()),
            Value::Bool(mute),
        ])
    }

    /// 设置播放速度
    pub fn set_speed(speed: f64) -> Self {
        Self::new_with_args("set_property", vec![
            Value::String("speed".to_string()),
            Value::Number(serde_json::Number::from_f64(speed).unwrap()),
        ])
    }

    /// 循环模式: "no", "inf", "file"
    pub fn set_loop(mode: &str) -> Self {
        Self::new_with_args("set_property", vec![
            Value::String("loop".to_string()),
            Value::String(mode.to_string()),
        ])
    }

    // ── 属性查询 ──

    /// 获取单个属性值
    pub fn get_property(name: &str) -> Self {
        Self::new_with_args("get_property", vec![Value::String(name.to_string())])
    }

    /// 观察属性变化（异步通知）
    pub fn observe_property(id: u64, name: &str) -> Self {
        Self::new_with_args("observe_property", vec![
            Value::Number(serde_json::Number::from(id)),
            Value::String(name.to_string()),
        ])
    }

    /// 取消观察
    pub fn unobserve_property(id: u64) -> Self {
        Self::new_with_args("unobserve_property", vec![
            Value::Number(serde_json::Number::from(id)),
        ])
    }

    // ── 播放列表 ──

    /// 清空播放列表
    pub fn playlist_clear() -> Self {
        Self::new("playlist-clear")
    }

    /// 下一首
    pub fn playlist_next() -> Self {
        Self::new("playlist-next")
    }

    /// 上一首
    pub fn playlist_prev() -> Self {
        Self::new("playlist-prev")
    }

    // ── OSD 显示 ──

    /// 显示 OSD 文本
    pub fn show_text(text: &str, duration_ms: u32) -> Self {
        Self::new_with_args("show-text", vec![
            Value::String(text.to_string()),
            Value::Number(serde_json::Number::from(duration_ms)),
        ])
    }

    /// 序列化为 JSON 字符串（每行一条命令，mpv 的 IPC 协议格式）
    pub fn to_ipc_message(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ──────────────────────────────────────────────
// MPV IPC 响应 / 事件
// ──────────────────────────────────────────────

/// MPV 事件类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MpvEventType {
    StartFile,
    EndFile,
    FileLoaded,
    Seek,
    PlaybackRestart,
    Pause,
    Unpause,
    Idle,
    Tick,
    Shutdown,
    VideoReconfig,
    AudioReconfig,
    PropertyChange,
    #[serde(other)]
    Unknown,
}

/// MPV IPC 响应/事件
#[derive(Debug, Clone, Deserialize)]
pub struct MpvEvent {
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
    /// 请求 ID（用于匹配命令和响应）
    #[serde(default)]
    pub request_id: Option<u64>,
}

/// MPV 属性变更事件
#[derive(Debug, Clone, Deserialize)]
pub struct PropertyChangeEvent {
    pub name: Option<String>,
    pub data: Option<Value>,
    pub id: Option<u64>,
}

/// 播放状态快照
#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    /// 当前播放位置（秒）
    pub time_pos: Option<f64>,
    /// 总时长（秒）
    pub duration: Option<f64>,
    /// 是否暂停
    pub paused: bool,
    /// 音量 (0-100)
    pub volume: f64,
    /// 是否静音
    pub muted: bool,
    /// 播放速度
    pub speed: f64,
    /// 当前文件路径
    pub filename: Option<String>,
    /// 是否正在播放（非 idle 状态）
    pub is_playing: bool,
}

impl PlaybackState {
    /// 播放进度百分比 (0-100)
    pub fn progress_percent(&self) -> f32 {
        match (self.time_pos, self.duration) {
            (Some(pos), Some(dur)) if dur > 0.0 => {
                ((pos / dur) * 100.0).clamp(0.0, 100.0) as f32
            }
            _ => 0.0,
        }
    }

    /// 当前时间显示 (MM:SS)
    pub fn time_display(&self) -> String {
        match self.time_pos {
            Some(t) => {
                let m = (t / 60.0) as u64;
                let s = (t % 60.0) as u64;
                format!("{:02}:{:02}", m, s)
            }
            None => "00:00".to_string(),
        }
    }

    /// 总时长显示 (MM:SS)
    pub fn duration_display(&self) -> String {
        match self.duration {
            Some(d) => {
                let m = (d / 60.0) as u64;
                let s = (d % 60.0) as u64;
                format!("{:02}:{:02}", m, s)
            }
            None => "00:00".to_string(),
        }
    }
}

// ──────────────────────────────────────────────
// 常用属性名称常量
// ──────────────────────────────────────────────

pub mod properties {
    pub const TIME_POS: &str = "time-pos";
    pub const DURATION: &str = "duration";
    pub const PAUSE: &str = "pause";
    pub const VOLUME: &str = "volume";
    pub const MUTE: &str = "mute";
    pub const SPEED: &str = "speed";
    pub const FILENAME: &str = "filename";
    pub const PATH: &str = "path";
    pub const VIDEO_CODEC: &str = "video-codec";
    pub const AUDIO_CODEC: &str = "audio-codec";
    pub const WIDTH: &str = "width";
    pub const HEIGHT: &str = "height";
    pub const FPS: &str = "container-fps";
}
