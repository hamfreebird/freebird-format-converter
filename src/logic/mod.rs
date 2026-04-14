pub mod execute;
pub mod dependent;

use regex::Regex;
use std::io::BufRead;
use std::process::Command;

#[derive(Debug, Clone)]
pub(crate) struct EncoderInfo {
    pub(crate) name: String,        // 编码器名称
    pub(crate) description: String, // 编码器描述
    pub(crate) is_video: bool,      // 是否为视频编码器
    pub(crate) is_audio: bool,      // 是否为音频编码器
    pub(crate) is_subtitle: bool,   // 是否为字幕编码器
    pub(crate) is_frame_multithreading: bool,
    pub(crate) is_slice_multithreading: bool,
    pub(crate) is_experimental: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FormatInfo {
    pub(crate) name: String,        // 格式简称，如 "mp4"
    pub(crate) description: String, // 格式全称，如 "MP4 (MPEG-4 Part 14)"
    pub(crate) can_mux: bool,       // 是否支持复用 (输出)
    pub(crate) can_demux: bool,     // 是否支持解复用 (输入)
}

#[derive(Debug, Clone)]
pub(crate) struct PixelFormatInfo {
    pub(crate) name: String,        // 像素格式名称，如 "yuv420p"
    pub(crate) input_ok: bool,      // 是否支持作为输入
    pub(crate) output_ok: bool,     // 是否支持作为输出
    pub(crate) bits_per_pixel: u32, // 每像素比特数
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
