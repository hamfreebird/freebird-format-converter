use std::path::{Path, PathBuf};
use std::process::Command;

/// 时间位置（用于剪辑起止点）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimePosition {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: f64,
}

impl TimePosition {
    /// 从总秒数创建
    pub fn from_secs(total_secs: f64) -> Self {
        let h = (total_secs / 3600.0) as u32;
        let m = ((total_secs % 3600.0) / 60.0) as u32;
        let s = total_secs % 60.0;
        Self {
            hours: h,
            minutes: m,
            seconds: s,
        }
    }

    /// 总秒数
    pub fn total_secs(&self) -> f64 {
        self.hours as f64 * 3600.0 + self.minutes as f64 * 60.0 + self.seconds
    }

    /// 解析时间字符串，支持以下格式：
    /// - "HH:MM:SS.ms" (01:23:45.678)
    /// - "MM:SS.ms" (23:45.678)
    /// - "SS.ms" (45.678)
    /// - 纯秒数 "123.456"
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty time string".to_string());
        }

        // 尝试解析纯秒数
        if let Ok(secs) = s.parse::<f64>() {
            return Ok(Self::from_secs(secs));
        }

        // 解析 HH:MM:SS.ms 格式
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            3 => {
                let h = parts[0]
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid hours: {}", parts[0]))?;
                let m = parts[1]
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid minutes: {}", parts[1]))?;
                let sec = parts[2]
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid seconds: {}", parts[2]))?;
                Ok(Self {
                    hours: h,
                    minutes: m,
                    seconds: sec,
                })
            }
            2 => {
                let m = parts[0]
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid minutes: {}", parts[0]))?;
                let sec = parts[1]
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid seconds: {}", parts[1]))?;
                Ok(Self {
                    hours: 0,
                    minutes: m,
                    seconds: sec,
                })
            }
            1 => {
                let sec = parts[0]
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid seconds: {}", parts[0]))?;
                Ok(Self::from_secs(sec))
            }
            _ => Err(format!("Invalid time format: {}", s)),
        }
    }

    /// FFmpeg 时间格式
    pub fn to_ffmpeg(&self) -> String {
        format!(
            "{:02}:{:02}:{:.3}",
            self.hours, self.minutes, self.seconds
        )
    }
}

impl std::fmt::Display for TimePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_ffmpeg())
    }
}

/// 剪辑模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClipMode {
    /// 无损剪辑（从关键帧切割，速度快但起止点可能不精确）
    Lossless,
    /// 有损剪辑（重新编码，精确到帧但较慢）
    Lossy,
}

/// 剪辑请求参数
#[derive(Debug, Clone)]
pub struct ClipRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub start: TimePosition,
    pub end: TimePosition,
    pub mode: ClipMode,
    /// 视频编码器（有损模式使用，如 "libx264"）
    pub video_encoder: Option<String>,
    /// 音频编码器（有损模式使用，如 "aac"）
    pub audio_encoder: Option<String>,
    /// CRF 质量（有损模式，0-51）
    pub crf: Option<u32>,
}

impl ClipRequest {
    /// 验证剪辑参数
    pub fn validate(&self) -> Result<(), String> {
        if !self.input_path.exists() {
            return Err(format!("Input file not found: {:?}", self.input_path));
        }
        if self.end.total_secs() <= self.start.total_secs() {
            return Err(format!(
                "End time ({}) must be after start time ({})",
                self.end, self.start
            ));
        }
        if self.mode == ClipMode::Lossy {
            if self.video_encoder.is_none() && self.audio_encoder.is_none() {
                return Err("Lossy mode requires at least one encoder".to_string());
            }
        }
        Ok(())
    }

    /// 剪辑时长（秒）
    pub fn duration_secs(&self) -> f64 {
        self.end.total_secs() - self.start.total_secs()
    }
}

/// 构建 FFmpeg 剪辑命令
pub fn build_clip_command(req: &ClipRequest) -> Result<Command, String> {
    req.validate()?;

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");

    // 输入定位：-ss 放在 -i 之前可快速 seek，但起点必须从关键帧开始
    cmd.arg("-ss").arg(req.start.to_ffmpeg());
    cmd.arg("-i").arg(&req.input_path);

    // 持续时间
    let duration = req.duration_secs();
    cmd.arg("-t").arg(format!("{:.3}", duration));

    match req.mode {
        ClipMode::Lossless => {
            // 无损模式：-c copy，起止点自动对齐到关键帧
            cmd.arg("-c").arg("copy");
            // -avoid_negative_ts 确保输出时间戳正确
            cmd.arg("-avoid_negative_ts").arg("make_zero");
        }
        ClipMode::Lossy => {
            // 有损模式：重新编码
            if let Some(ref enc) = req.video_encoder {
                cmd.arg("-c:v").arg(enc);
            } else {
                cmd.arg("-c:v").arg("copy");
            }
            if let Some(ref enc) = req.audio_encoder {
                cmd.arg("-c:a").arg(enc);
            } else {
                cmd.arg("-c:a").arg("copy");
            }
            if let Some(crf) = req.crf {
                cmd.arg("-crf").arg(crf.to_string());
            }
        }
    }

    cmd.arg(&req.output_path);
    Ok(cmd)
}

/// 快速剪辑（seek 在输入前，适合长视频快速定位）
pub fn build_fast_clip_command(req: &ClipRequest) -> Result<Command, String> {
    req.validate()?;

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_border");
    cmd.arg("-ss").arg(req.start.to_ffmpeg());
    cmd.arg("-i").arg(&req.input_path);
    cmd.arg("-to").arg(req.end.to_ffmpeg());

    match req.mode {
        ClipMode::Lossless => {
            cmd.arg("-c").arg("copy");
            cmd.arg("-avoid_negative_ts").arg("make_zero");
        }
        ClipMode::Lossy => {
            if let Some(ref enc) = req.video_encoder {
                cmd.arg("-c:v").arg(enc);
            } else {
                cmd.arg("-c:v").arg("copy");
            }
            if let Some(ref enc) = req.audio_encoder {
                cmd.arg("-c:a").arg(enc);
            } else {
                cmd.arg("-c:a").arg("copy");
            }
            if let Some(crf) = req.crf {
                cmd.arg("-crf").arg(crf.to_string());
            }
        }
    }

    cmd.arg(&req.output_path);
    Ok(cmd)
}

/// 提取视频缩略图（在指定时间点截图）
pub fn extract_thumbnail(
    input: &Path,
    output: &Path,
    at_time: TimePosition,
    width: Option<u32>,
) -> Result<Command, String> {
    if !input.exists() {
        return Err(format!("Input file not found: {:?}", input));
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");
    cmd.arg("-ss").arg(at_time.to_ffmpeg());
    cmd.arg("-i").arg(input);
    cmd.arg("-vframes").arg("1");
    cmd.arg("-q:v").arg("2");

    if let Some(w) = width {
        cmd.arg("-vf").arg(format!("scale={}:-1", w));
    }

    cmd.arg(output);
    Ok(cmd)
}
