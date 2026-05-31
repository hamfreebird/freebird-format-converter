use std::path::{Path, PathBuf};

use crate::core::processor::ffmpeg::{build_ffmpeg_command, validate_transcode_params};
use crate::core::task::TranscodeParams;
use crate::core::utils::metadata::extract_metadata;

/// 单文件转换请求（UI → 业务层）
#[derive(Debug, Clone)]
pub struct ConversionRequest {
    pub encoder: String,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_subtitle: bool,
    pub container: String,
    pub pix_fmt: String,
    pub bitrate: String,
    pub quality: String,
    pub preset: String,
    pub gop: String,
    pub input_path: PathBuf,
    pub output_dir: Option<PathBuf>,
}

impl ConversionRequest {
    /// 验证参数有效性
    pub fn validate(&self) -> Result<(), String> {
        validate_transcode_params(
            &self.encoder,
            self.is_video,
            self.is_audio,
            self.is_subtitle,
            &self.container,
            &self.pix_fmt,
            &self.bitrate,
            &self.quality,
            &self.gop,
            Some(&self.input_path),
            self.output_dir.as_deref(),
        )
    }

    /// 转换为 TranscodeParams（用于提交到 TaskManager）
    pub fn into_transcode_params(self) -> TranscodeParams {
        TranscodeParams {
            encoder: self.encoder,
            is_video: self.is_video,
            is_audio: self.is_audio,
            is_subtitle: self.is_subtitle,
            container: self.container,
            pix_fmt: self.pix_fmt,
            bitrate: self.bitrate,
            quality: self.quality,
            preset: self.preset,
            gop: self.gop,
            input_path: self.input_path,
            output_dir: self.output_dir,
        }
    }

    /// 预览将要执行的 FFmpeg 命令（调试用）
    pub fn preview_command(&self) -> Result<String, String> {
        let cmd = build_ffmpeg_command(
            &self.encoder,
            self.is_video,
            self.is_audio,
            self.is_subtitle,
            &self.container,
            &self.pix_fmt,
            &self.bitrate,
            &self.quality,
            &self.preset,
            &self.gop,
            Some(&self.input_path),
            self.output_dir.as_deref(),
        )?;
        Ok(format!("{:?}", cmd))
    }

    /// 输出文件路径（推断）
    pub fn output_path(&self) -> PathBuf {
        let stem = self
            .input_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("output");
        let filename = format!("{}.{}", stem, self.container);
        match &self.output_dir {
            Some(dir) => dir.join(filename),
            None => self
                .input_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(filename),
        }
    }

    /// 输入文件名（用于 UI 展示）
    pub fn input_name(&self) -> &str {
        self.input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
    }
}

/// 扫描文件夹，为每个媒体文件生成 ConversionRequest
pub fn scan_folder_for_conversion(
    folder: &Path,
    request_template: &ConversionRequest,
) -> Result<Vec<ConversionRequest>, String> {
    if !folder.is_dir() {
        return Err(format!("Not a directory: {:?}", folder));
    }

    let supported_extensions = [
        "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
        "ts", "m2ts", "mts", "3gp", "ogv", "rm", "rmvb", "asf", "vob", "divx",
        "mp3", "aac", "flac", "wav", "ogg", "opus", "wma", "m4a", "ac3", "eac3",
    ];

    let mut requests = Vec::new();

    let entries = std::fs::read_dir(folder)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if supported_extensions.contains(&ext.to_lowercase().as_str()) {
                    let mut req = request_template.clone();
                    req.input_path = path;
                    requests.push(req);
                }
            }
        }
    }

    if requests.is_empty() {
        return Err("No supported media files found in the folder".to_string());
    }

    Ok(requests)
}

/// 为单个文件创建转换请求（便捷方法）
pub fn create_conversion_request(
    encoder: &str,
    is_video: bool,
    is_audio: bool,
    is_subtitle: bool,
    container: &str,
    pix_fmt: &str,
    bitrate: &str,
    quality: &str,
    preset: &str,
    gop: &str,
    input_path: PathBuf,
    output_dir: Option<PathBuf>,
) -> ConversionRequest {
    ConversionRequest {
        encoder: encoder.to_string(),
        is_video,
        is_audio,
        is_subtitle,
        container: container.to_string(),
        pix_fmt: pix_fmt.to_string(),
        bitrate: bitrate.to_string(),
        quality: quality.to_string(),
        preset: preset.to_string(),
        gop: gop.to_string(),
        input_path,
        output_dir,
    }
}

/// 检查文件的元数据并推荐合适的编码器
pub fn recommend_encoder(file_path: &Path) -> Option<String> {
    let meta = extract_metadata(file_path).ok()?;

    // 基于输入文件的编码器推荐
    if let Some(video) = meta.video_streams.first() {
        match video.codec_name.as_str() {
            // 已经是高效编码器，保留原始编码器
            "hevc" | "h265" => return Some("libx265".to_string()),
            "av1" => return Some("libsvtav1".to_string()),
            "vp9" => return Some("libvpx-vp9".to_string()),
            // 常见 H.264 → 推荐 x264（最通用）
            "h264" => return Some("libx264".to_string()),
            // 老旧编码器 → 推荐升级
            "mpeg4" | "msmpeg4" | "wmv1" | "wmv2" | "mpeg2video" | "mpeg1video" => {
                return Some("libx264".to_string())
            }
            _ => return Some("libx264".to_string()),
        }
    }

    // 仅音频文件
    if meta.has_audio() && !meta.has_video() {
        return Some("libopus".to_string());
    }

    None
}
