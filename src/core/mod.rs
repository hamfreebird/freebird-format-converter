pub mod task;
pub mod processor;
pub mod player;
pub mod utils;

#[derive(Debug)]
enum VideoEncoderClass {
    // 软件编码器
    SoftwareX264,
    SoftwareX265,
    SoftwareVpx,
    SoftwareAom,
    SoftwareSvtAv1,
    SoftwareRav1e,
    SoftwareTheora,
    // 硬件编码器
    NvidiaNvenc,
    IntelQsv,
    AmdAmf,
    AppleVideotoolbox,
    Vaapi,          // VA-API (Linux)
    // 其他
    Mjpeg,
    Wmv,
    Msmpeg4,
    H263,
    Other,
}

#[derive(Debug)]
enum AudioEncoderClass {
    // 有损编码器
    Libmp3lame,
    Aac,
    LibfdkAac,
    Libopus,
    Libvorbis,
    Ac3,
    Eac3,
    Libtwolame,     // MP2
    Libshine,       // MP3 固定质量
    Libspeex,
    Libgsm,
    Libilbc,
    G722,
    G726,
    // 无损/近无损
    Flac,
    Alac,
    Pcm,            // PCM 系列 (pcm_s16le, pcm_s24le 等)
    // 其他
    Other,
}

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