use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// 视频流信息
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VideoStreamInfo {
    /// 编码器名称 (h264, hevc, vp9, ...)
    #[serde(default)]
    pub codec_name: String,
    /// 编码器全名
    #[serde(default)]
    pub codec_long_name: String,
    /// 宽度
    #[serde(default)]
    pub width: u32,
    /// 高度
    #[serde(default)]
    pub height: u32,
    /// 帧率（原始字符串，如 "30000/1001"）
    #[serde(default)]
    pub r_frame_rate: String,
    /// 平均帧率
    #[serde(default)]
    pub avg_frame_rate: String,
    /// 像素格式
    #[serde(default)]
    pub pix_fmt: String,
    /// 比特率 (bps)
    #[serde(default, rename = "bit_rate")]
    pub bit_rate_str: String,
    /// 时长（秒，字符串）
    #[serde(default)]
    pub duration: String,
}

impl VideoStreamInfo {
    /// 比特率 (bps)，解析失败返回 0
    pub fn bit_rate(&self) -> u64 {
        self.bit_rate_str.parse().unwrap_or(0)
    }

    /// 时长（秒），解析失败返回 0.0
    pub fn duration_secs(&self) -> f64 {
        self.duration.parse().unwrap_or(0.0)
    }

    /// 帧率，解析 "30000/1001" 格式，失败返回 0.0
    pub fn frame_rate(&self) -> f64 {
        parse_frac(&self.r_frame_rate).or_else(|| parse_frac(&self.avg_frame_rate)).unwrap_or(0.0)
    }

    /// 分辨率描述文本
    pub fn resolution_display(&self) -> String {
        if self.width > 0 && self.height > 0 {
            format!("{}×{}", self.width, self.height)
        } else {
            "Unknown".to_string()
        }
    }
}

/// 音频流信息
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AudioStreamInfo {
    /// 编码器名称 (aac, mp3, opus, ...)
    #[serde(default)]
    pub codec_name: String,
    /// 编码器全名
    #[serde(default)]
    pub codec_long_name: String,
    /// 采样率 (Hz)
    #[serde(default)]
    pub sample_rate_str: String,
    /// 声道数
    #[serde(default)]
    pub channels: u8,
    /// 声道布局
    #[serde(default)]
    pub channel_layout: String,
    /// 比特率 (bps)
    #[serde(default, rename = "bit_rate")]
    pub bit_rate_str: String,
    /// 时长（秒）
    #[serde(default)]
    pub duration: String,
    /// 语言标签
    #[serde(default)]
    pub tags: StreamTags,
}

impl AudioStreamInfo {
    /// 采样率 (Hz)，解析失败返回 0
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate_str.parse().unwrap_or(0)
    }

    /// 比特率 (bps)，解析失败返回 0
    pub fn bit_rate(&self) -> u64 {
        self.bit_rate_str.parse().unwrap_or(0)
    }

    /// 时长（秒），解析失败返回 0.0
    pub fn duration_secs(&self) -> f64 {
        self.duration.parse().unwrap_or(0.0)
    }

    /// 音频描述文本
    pub fn display(&self) -> String {
        let lang = if self.tags.language.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.tags.language)
        };
        format!(
            "{} | {} Hz | {}ch{}",
            self.codec_name,
            self.sample_rate(),
            self.channels,
            lang
        )
    }
}

/// 字幕流信息
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SubtitleStreamInfo {
    /// 编码器名称
    #[serde(default)]
    pub codec_name: String,
    /// 语言标签
    #[serde(default)]
    pub tags: StreamTags,
}

impl SubtitleStreamInfo {
    pub fn display(&self) -> String {
        let lang = if self.tags.language.is_empty() {
            "unknown"
        } else {
            &self.tags.language
        };
        format!("{} [{}]", self.codec_name, lang)
    }
}

/// 流标签（语言等）
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StreamTags {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub title: String,
}

/// 容器格式信息
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FormatInfoMeta {
    /// 文件名
    #[serde(default)]
    pub filename: String,
    /// 格式名（如 "mov,mp4,m4a,3gp,3g2,mj2"）
    #[serde(default)]
    pub format_name: String,
    /// 时长（秒）
    #[serde(default)]
    pub duration: String,
    /// 文件大小（字节）
    #[serde(default)]
    pub size_str: String,
    /// 总比特率 (bps)
    #[serde(default, rename = "bit_rate")]
    pub bit_rate_str: String,
}

impl FormatInfoMeta {
    /// 时长（秒），解析失败返回 0.0
    pub fn duration_secs(&self) -> f64 {
        self.duration.parse().unwrap_or(0.0)
    }

    /// 文件大小（字节），解析失败返回 0
    pub fn size(&self) -> u64 {
        self.size_str.parse().unwrap_or(0)
    }

    /// 比特率 (bps)，解析失败返回 0
    pub fn bit_rate(&self) -> u64 {
        self.bit_rate_str.parse().unwrap_or(0)
    }

    /// 人类可读的文件大小
    pub fn size_display(&self) -> String {
        let s = self.size();
        if s >= 1_000_000_000 {
            format!("{:.2} GB", s as f64 / 1_000_000_000.0)
        } else if s >= 1_000_000 {
            format!("{:.2} MB", s as f64 / 1_000_000.0)
        } else if s >= 1_000 {
            format!("{:.2} KB", s as f64 / 1_000.0)
        } else {
            format!("{} B", s)
        }
    }
}

/// ffprobe 原始 JSON 结构（用于 serde 反序列化）
#[derive(Debug, Deserialize)]
struct FFprobeOutput {
    #[serde(default)]
    streams: Vec<FFprobeStream>,
    #[serde(default)]
    format: Option<FFprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FFprobeStream {
    #[serde(default)]
    codec_type: String,
    #[serde(default)]
    codec_name: String,
    #[serde(default)]
    codec_long_name: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    r_frame_rate: String,
    #[serde(default)]
    avg_frame_rate: String,
    #[serde(default)]
    pix_fmt: String,
    #[serde(default)]
    sample_rate: String,
    #[serde(default)]
    channels: u8,
    #[serde(default)]
    channel_layout: String,
    #[serde(default)]
    bit_rate: String,
    #[serde(default)]
    duration: String,
    #[serde(default)]
    tags: Option<FFprobeTags>,
}

#[derive(Debug, Deserialize, Default)]
struct FFprobeTags {
    #[serde(default)]
    language: String,
    #[serde(default)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct FFprobeFormat {
    #[serde(default)]
    filename: String,
    #[serde(default)]
    format_name: String,
    #[serde(default)]
    duration: String,
    #[serde(default)]
    size: String,
    #[serde(default)]
    bit_rate: String,
}

/// 提取媒体文件的元数据
pub fn extract_metadata(input_path: &Path) -> Result<MediaMetadata, String> {
    if !input_path.exists() {
        return Err(format!("File not found: {:?}", input_path));
    }

    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input_path)
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: FFprobeOutput =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse ffprobe JSON: {}", e))?;

    Ok(build_metadata(parsed))
}

fn build_metadata(raw: FFprobeOutput) -> MediaMetadata {
    let mut meta = MediaMetadata::default();

    for stream in raw.streams {
        let tags = stream.tags.unwrap_or_default();
        match stream.codec_type.as_str() {
            "video" => {
                meta.video_streams.push(VideoStreamInfo {
                    codec_name: stream.codec_name,
                    codec_long_name: stream.codec_long_name,
                    width: stream.width,
                    height: stream.height,
                    r_frame_rate: stream.r_frame_rate,
                    avg_frame_rate: stream.avg_frame_rate,
                    pix_fmt: stream.pix_fmt,
                    bit_rate_str: stream.bit_rate,
                    duration: stream.duration,
                });
            }
            "audio" => {
                meta.audio_streams.push(AudioStreamInfo {
                    codec_name: stream.codec_name,
                    codec_long_name: stream.codec_long_name,
                    sample_rate_str: stream.sample_rate,
                    channels: stream.channels,
                    channel_layout: stream.channel_layout,
                    bit_rate_str: stream.bit_rate,
                    duration: stream.duration,
                    tags: StreamTags {
                        language: tags.language,
                        title: tags.title,
                    },
                });
            }
            "subtitle" => {
                meta.subtitle_streams.push(SubtitleStreamInfo {
                    codec_name: stream.codec_name,
                    tags: StreamTags {
                        language: tags.language,
                        title: tags.title,
                    },
                });
            }
            _ => {}
        }
    }

    if let Some(fmt) = raw.format {
        meta.format = FormatInfoMeta {
            filename: fmt.filename,
            format_name: fmt.format_name,
            duration: fmt.duration,
            size_str: fmt.size,
            bit_rate_str: fmt.bit_rate,
        };
    }

    meta
}

/// 解析分数格式的帧率字符串（如 "30000/1001" → 29.97）
fn parse_frac(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().ok()?;
        let den: f64 = parts[1].parse().ok()?;
        if den != 0.0 {
            Some(num / den)
        } else {
            None
        }
    } else {
        s.parse().ok()
    }
}

// ──────────────────────────────────────────────────────
// 公开的元数据结构
// ──────────────────────────────────────────────────────

/// 完整的媒体文件元数据
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    pub format: FormatInfoMeta,
}

impl MediaMetadata {
    /// 是否有视频流
    pub fn has_video(&self) -> bool {
        !self.video_streams.is_empty()
    }

    /// 是否有音频流
    pub fn has_audio(&self) -> bool {
        !self.audio_streams.is_empty()
    }

    /// 时长（秒）
    pub fn duration_secs(&self) -> f64 {
        self.format.duration_secs()
    }

    /// 时长显示（HH:MM:SS）
    pub fn duration_display(&self) -> String {
        let secs = self.duration_secs();
        let h = (secs / 3600.0) as u64;
        let m = ((secs % 3600.0) / 60.0) as u64;
        let s = (secs % 60.0) as u64;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }

    /// 简要摘要（用于 UI 快速预览）
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(v) = self.video_streams.first() {
            parts.push(format!(
                "Video: {} {} {}",
                v.codec_name,
                v.resolution_display(),
                if v.frame_rate() > 0.0 {
                    format!("{:.2}fps", v.frame_rate())
                } else {
                    String::new()
                }
            ));
        }

        if let Some(a) = self.audio_streams.first() {
            parts.push(format!("Audio: {}", a.display()));
        }

        if !self.subtitle_streams.is_empty() {
            parts.push(format!("Subtitles: {} track(s)", self.subtitle_streams.len()));
        }

        if self.duration_secs() > 0.0 {
            parts.push(format!("Duration: {}", self.duration_display()));
        }

        parts.join(" | ")
    }
}
