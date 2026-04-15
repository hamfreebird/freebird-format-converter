use crate::core::{AudioEncoderClass, EncoderInfo, FormatInfo, PixelFormatInfo, VideoEncoderClass};
use regex::Regex;
use std::io::BufRead;
use std::path::Path;
use std::process::Command;

impl VideoEncoderClass {
    fn from_name(name: &str) -> Self {
        match name {
            "libx264" | "libx264rgb" => Self::SoftwareX264,
            "libx265" => Self::SoftwareX265,
            "libvpx" | "libvpx-vp9" => Self::SoftwareVpx,
            "libaom-av1" => Self::SoftwareAom,
            "libsvtav1" => Self::SoftwareSvtAv1,
            "librav1e" => Self::SoftwareRav1e,
            "libtheora" => Self::SoftwareTheora,
            n if n.contains("nvenc") => Self::NvidiaNvenc,
            n if n.contains("qsv") => Self::IntelQsv,
            n if n.contains("amf") => Self::AmdAmf,
            n if n.contains("videotoolbox") => Self::AppleVideotoolbox,
            n if n.contains("vaapi") => Self::Vaapi,
            "mjpeg" | "mjpeg_qsv" => Self::Mjpeg,
            n if n.starts_with("wmv") => Self::Wmv,
            n if n == "msmpeg4" || n == "msmpeg4v2" || n == "msmpeg4v3" => Self::Msmpeg4,
            n if n == "h263" || n == "h263p" => Self::H263,
            _ => Self::Other,
        }
    }

    /// 质量参数名（-crf / -cq / -qp / -q:v 等）
    fn quality_param(&self) -> &'static str {
        match self {
            Self::SoftwareX264 | Self::SoftwareX265 | Self::SoftwareVpx | Self::SoftwareAom | Self::SoftwareSvtAv1 => "-crf",
            Self::SoftwareRav1e => "-qp",           // rav1e 使用 -qp
            Self::SoftwareTheora => "-q:v",         // theora 使用 -q:v
            Self::NvidiaNvenc => "-cq",
            Self::IntelQsv => "-global_quality",
            Self::AmdAmf => "-qp_i",
            Self::AppleVideotoolbox => "-quality",
            Self::Vaapi => "-qp",                   // VA-API 使用 -qp
            Self::Mjpeg => "-q:v",
            Self::Wmv | Self::Msmpeg4 | Self::H263 => "-q:v", // 这些编码器也支持 -q:v
            Self::Other => "-crf",
        }
    }

    /// 预设参数名（-preset / -quality / -compression_level 等）
    fn preset_param(&self) -> &'static str {
        match self {
            Self::AmdAmf => "-quality",
            Self::AppleVideotoolbox => "-preset",
            Self::Vaapi => "-compression_level",    // VA-API 预设用 -compression_level
            Self::Mjpeg => "-compression_level",
            Self::Wmv | Self::Msmpeg4 | Self::H263 => "-q:v", // 不严格区分预设，可用质量代替
            _ => "-preset",
        }
    }
}

impl AudioEncoderClass {
    fn from_name(name: &str) -> Self {
        match name {
            "libmp3lame" => Self::Libmp3lame,
            "aac" => Self::Aac,
            "libfdk_aac" => Self::LibfdkAac,
            "libopus" => Self::Libopus,
            "libvorbis" => Self::Libvorbis,
            "ac3" => Self::Ac3,
            "eac3" => Self::Eac3,
            "libtwolame" => Self::Libtwolame,
            "libshine" => Self::Libshine,
            "libspeex" => Self::Libspeex,
            "libgsm" => Self::Libgsm,
            "libilbc" => Self::Libilbc,
            "g722" => Self::G722,
            "g726" => Self::G726,
            "flac" => Self::Flac,
            "alac" => Self::Alac,
            n if n.starts_with("pcm_") => Self::Pcm,
            _ => Self::Other,
        }
    }

    /// 质量参数名（有些编码器使用 -q:a，有些使用 -vbr 或 -compression_level）
    fn quality_param(&self) -> Option<&'static str> {
        match self {
            Self::Libopus => Some("-vbr"),
            Self::LibfdkAac => Some("-vbr"),
            Self::Flac => Some("-compression_level"),
            Self::Alac => Some("-compression_level"),
            Self::Libshine => Some("-q"),
            Self::Libspeex => Some("-q"),
            Self::G722 | Self::G726 => None,   // 这些编码器通常不支持可变质量
            Self::Pcm => None,                  // PCM 无损，无需质量
            _ => Some("-q:a"),
        }
    }

    /// 是否支持传统的 -q:a 数值范围
    fn supports_q_scale(&self) -> bool {
        matches!(
            self,
            Self::Libmp3lame | Self::Aac | Self::Libvorbis | Self::Libtwolame | Self::Other
        )
    }

    /// 音频预设参数名（部分编码器支持 -compression_level 等）
    fn preset_param(&self) -> Option<&'static str> {
        match self {
            Self::Libopus => Some("-compression_level"),
            Self::Flac => Some("-compression_level"),
            Self::Alac => Some("-compression_level"),
            Self::Libvorbis => Some("-q:a"),  // vorbis 预设直接使用质量
            _ => None,
        }
    }
}

/// 构建 ffmpeg 命令行，支持视频/音频/字幕编码器（更 多 的 编 码 器！！！）
/// - --- ☆*: .｡. o(≧▽≦)o .｡.:*☆ ---
///
/// # 参数
/// - `encoder`: 编码器名称
/// - `is_video`, `is_audio`, `is_subtitle`: 编码器类型标志
/// - `container`: 容器扩展名（如 "mp4"）
/// - `pix_fmt`: 像素格式（仅视频）
/// - `bitrate`: 比特率，视频时使用 `-b:v`，音频时使用 `-b:a`
/// - `quality`: 恒定质量/质量值（视频映射为 CRF/CQ 等，音频映射为 -q:a 或专用参数）
/// - `preset`: 预设（视频映射为 -preset 或 -quality，音频映射为压缩等级等）
/// - `gop`: GOP 间隔（仅视频）
/// - `input_path`: 输入文件路径（必须）
/// - `output_dir`: 输出文件夹路径（可选，默认输入文件所在目录）
pub fn build_ffmpeg_command(
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
    input_path: Option<&Path>,
    output_dir: Option<&Path>,
) -> Result<Command, String> {
    let input = input_path.ok_or("The input file path cannot be empty")?;
    if !input.exists() {
        return Err(format!("The input file doesn't exist: {:?}", input));
    }

    // 构造输出文件路径
    let stem = input.file_stem().ok_or("Invalid input file name")?;
    let output_filename = format!("{}.{}", stem.to_string_lossy(), container);
    let output_path = match output_dir {
        Some(dir) => dir.join(output_filename),
        None => input.parent().unwrap_or(Path::new(".")).join(output_filename),
    };

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");
    cmd.arg("-i").arg(input);

    // 设置编码器（根据类型选择流）
    if is_video {
        cmd.arg("-c:v").arg(encoder);
    }
    if is_audio {
        cmd.arg("-c:a").arg(encoder);
    }
    if is_subtitle {
        cmd.arg("-c:s").arg(encoder);
    }

    // 视频专用参数
    if is_video {
        if !pix_fmt.is_empty() {
            cmd.arg("-pix_fmt").arg(pix_fmt);
        }
        if !gop.is_empty() {
            cmd.arg("-g").arg(gop);
        }
        let video_class = VideoEncoderClass::from_name(encoder);
        // 比特率或质量二选一
        let use_quality = !quality.trim().is_empty();
        if use_quality {
            cmd.arg(video_class.quality_param()).arg(quality);
        } else if !bitrate.trim().is_empty() {
            cmd.arg("-b:v").arg(bitrate);
        }
        if !preset.trim().is_empty() {
            cmd.arg(video_class.preset_param()).arg(preset);
        }
    }

    // 音频专用参数
    if is_audio {
        let audio_class = AudioEncoderClass::from_name(encoder);
        let use_quality = !quality.trim().is_empty();
        if use_quality {
            if let Some(param) = audio_class.quality_param() {
                cmd.arg(param).arg(quality);
            } else if audio_class.supports_q_scale() {
                cmd.arg("-q:a").arg(quality);
            }
            // 对于 G722/G726/PCM，忽略质量参数（它们不支持）
        } else if !bitrate.trim().is_empty() {
            cmd.arg("-b:a").arg(bitrate);
        }

        // 音频预设处理
        if !preset.trim().is_empty() {
            if let Some(param) = audio_class.preset_param() {
                cmd.arg(param).arg(preset);
            } else {
                // 对于其他编码器，如果预设非空且未映射，可以忽略或转为 -q:a
                // 这里保持静默忽略
            }
        }
    }

    // 字幕编码器无额外参数

    cmd.arg(output_path);
    Ok(cmd)
}

/// 检查转码参数的有效性
///
/// # 返回
/// - `Ok(())` 所有参数有效
/// - `Err(String)` 无效参数及其原因
pub(crate) fn validate_transcode_params(
    encoder: &str,
    is_video: bool,
    is_audio: bool,
    is_subtitle: bool,
    container: &str,
    pix_fmt: &str,
    bitrate: &str,
    quality: &str,
    gop: &str,
    input_path: Option<&Path>,
    output_dir: Option<&Path>,
) -> Result<(), String> {
    // 1. 编码器名称不能为空
    if encoder.trim().is_empty() {
        return Err("The encoder name cannot be empty".to_string());
    }

    // 2. 至少指定一种流类型
    if !is_video && !is_audio && !is_subtitle {
        return Err("Must specify at least one type of encoder in video, audio, or subtitles".to_string());
    }

    // 3. 容器格式不能为空
    if container.trim().is_empty() {
        return Err("The container format cannot be empty".to_string());
    }

    // 4. 如果是视频编码，像素格式建议不为空（但允许空，FFmpeg 会使用默认值）
    if is_video && pix_fmt.trim().is_empty() {
        eprintln!("Warning: no pixel format, ffmpeg will use the default pixel format");
    }

    // 5. 比特率与质量不能同时为空（可选严格限制，也可允许都为空让 FFmpeg 使用默认值）
    if bitrate.trim().is_empty() && quality.trim().is_empty() {
        eprintln!("Warning: no bitrate or quality, ffmpeg may use the encoder default");
        // 如果希望强制用户指定，可以取消下面注释：
        // return Err("必须提供比特率或质量参数".to_string());
    }

    // 6. 如果比特率非空，简单检查格式（可选）
    if !bitrate.trim().is_empty() {
        let br = bitrate.trim();
        if !(br.ends_with('k') || br.ends_with('K') || br.ends_with('M') || br.ends_with('G') || br.chars().all(|c| c.is_ascii_digit())) {
            eprintln!("Warning: bitrate format unconventional (recommended using digital heel k/m/g, such as 2m)");
        }
    }

    // 7. 如果质量非空，对于视频 CRF 通常为数字（可选检查）
    if is_video && !quality.trim().is_empty() {
        if quality.trim().parse::<f64>().is_err() {
            eprintln!("Warning: the video quality value should be the number (such as 23)");
        }
    }

    // 8. GOP 间隔如果非空，应为正整数
    if !gop.trim().is_empty() {
        if gop.trim().parse::<u32>().is_err() {
            return Err("GOP interval must be positive integers".to_string());
        }
    }

    // 9. 输入文件存在性（由 build_ffmpeg_command 内部检查，此处可选提前检查）
    if let Some(path) = input_path {
        if !path.exists() {
            return Err(format!("The input file doesn't exist: {:?}", path));
        }
    } else {
        return Err("The input file path cannot be empty".to_string());
    }

    // 10. 输出目录如果提供，应存在或可创建（此处只检查父目录是否存在，若不存在则尝试创建？）
    if let Some(dir) = output_dir {
        if !dir.exists() {
            // 可以选择自动创建，或者报错。这里仅警告
            eprintln!("Warning: the output directory does not exist and will try to create: {:?}", dir);
            // 若需要强制存在，可改为 return Err(...)
        }
    }

    Ok(())
}

// 获取所有可用的编码器
pub(crate) fn get_encoders() -> Vec<EncoderInfo> {
    let output = Command::new("ffmpeg")
        .arg("-encoders")
        .output()
        .expect("Failed to execute ffmpeg -encoders");

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_encoders_output(&stdout)
}

fn parse_encoders_output(output: &str) -> Vec<EncoderInfo> {
    let mut encoders = Vec::new();
    let re = Regex::new(r"^\s*([ VASFXBD.]+)\s+(\S+)\s+(.+)$").unwrap();

    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            let flags = &caps[1];
            let name = caps[2].to_string();
            let description = caps[3].trim().to_string();

            // Encoders:
            // V..... = Video
            // A..... = Audio
            // S..... = Subtitle 字幕编码器
            // .F.... = Frame-level multithreading 帧级多线程
            // ..S... = Slice-level multithreading 片级多线程
            // ...X.. = Codec is experimental      实验性编码器 需要-strict experimental
            // ....B. = Supports draw_horiz_band
            // .....D = Supports direct rendering method 1
            // 最后两个是优化特性不用管
            let is_video = flags.as_bytes()[0] == u8::try_from('V').unwrap();
            let is_audio = flags.as_bytes()[0] == u8::try_from('A').unwrap();
            let is_subtitle = flags.as_bytes()[0] == u8::try_from('S').unwrap();
            let is_frame_multithreading = flags.as_bytes()[1] == u8::try_from('F').unwrap();
            let is_slice_multithreading = flags.as_bytes()[2] == u8::try_from('S').unwrap();
            let is_experimental = flags.as_bytes()[3] == u8::try_from('X').unwrap();

            if (is_video || is_audio || is_subtitle)  & !(name == "=") {
                encoders.push(EncoderInfo {
                    name,
                    description,
                    is_video,
                    is_audio,
                    is_subtitle,
                    is_frame_multithreading,
                    is_slice_multithreading,
                    is_experimental,
                });
            }
        }
    }
    encoders
}

// 获取所有可用的容器格式
pub(crate) fn get_formats() -> Vec<FormatInfo> {
    let output = Command::new("ffmpeg")
        .arg("-formats")
        .output()
        .expect("Failed to execute ffmpeg -formats");

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_formats_output(&stdout)
}

fn parse_formats_output(output: &str) -> Vec<FormatInfo> {
    let mut formats = Vec::new();
    let re = Regex::new(r"^\s*([ DE]+)\s+(\S+)\s+(.+)$").unwrap();

    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            let flags = &caps[1];
            let name = caps[2].to_string();
            let description = caps[3].trim().to_string();

            // 标志位解析：'D' 表示可作为输入 (解复用)，'E' 表示可作为输出 (复用)
            let can_demux = flags.contains('D');
            let can_mux = flags.contains('E');

            formats.push(FormatInfo {
                name,
                description,
                can_mux,
                can_demux,
            });
        }
    }
    formats
}

// 获取所有可用的像素格式
pub(crate) fn get_pixel_formats() -> Vec<PixelFormatInfo> {
    let output = Command::new("ffmpeg")
        .arg("-pix_fmts")
        .output()
        .expect("Failed to execute ffmpeg -pix_fmts");

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pixel_formats_output(&stdout)
}

fn parse_pixel_formats_output(output: &str) -> Vec<PixelFormatInfo> {
    let mut formats = Vec::new();
    let re = Regex::new(r"^\s*([ IO.]+)\s+(\S+)\s+(\d+)\s+(\d+)").unwrap();

    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            let flags = &caps[1];
            let name = caps[2].to_string();
            let _nb_components = caps[3].parse::<u32>().unwrap_or(0);
            let bits_per_pixel = caps[4].parse::<u32>().unwrap_or(0);

            // 标志位解析：'I' 表示支持作为输入，'O' 表示支持作为输出
            let input_ok = flags.contains('I');
            let output_ok = flags.contains('O');

            formats.push(PixelFormatInfo {
                name,
                input_ok,
                output_ok,
                bits_per_pixel,
            });
        }
    }
    formats
}

// 获取所有可用的色彩名称
fn get_colors() -> Vec<String> {
    let output = Command::new("ffmpeg")
        .arg("-colors")
        .output()
        .expect("Failed to execute ffmpeg -colors");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().map(|s| s.trim().to_string()).collect()
}

