use std::path::PathBuf;
use std::process::Command;

/// HLS 切片配置
#[derive(Debug, Clone)]
pub struct HlsSliceConfig {
    /// 输入文件路径
    pub input_path: PathBuf,
    /// 输出目录
    pub output_dir: PathBuf,
    /// 输出文件名前缀（如 "output" → output.m3u8, output_001.ts）
    pub output_prefix: String,
    /// 每个 TS 分段的时长（秒）
    pub segment_duration: u32,
    /// HLS 播放列表包含的分段数（0 = 包含所有分段）
    pub playlist_size: u32,
    /// 比特率阶梯（多码率 HLS），为空则单码率
    pub bitrate_ladder: Vec<BitrateVariant>,
}

/// 单码率变体
#[derive(Debug, Clone)]
pub struct BitrateVariant {
    /// 分辨率宽度
    pub width: u32,
    /// 分辨率高度（0 = 自动计算）
    pub height: u32,
    /// 视频比特率（如 "2000k"）
    pub video_bitrate: String,
    /// 音频比特率（如 "128k"）
    pub audio_bitrate: String,
    /// 最大码率（如 "2400k"）
    pub max_bitrate: String,
    /// 带宽值（用于 #EXT-X-STREAM-INF）
    pub bandwidth: u64,
}

impl Default for HlsSliceConfig {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            output_dir: PathBuf::from("."),
            output_prefix: "output".to_string(),
            segment_duration: 6,
            playlist_size: 0,
            bitrate_ladder: Vec::new(),
        }
    }
}

impl HlsSliceConfig {
    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if !self.input_path.exists() {
            return Err(format!("Input file not found: {:?}", self.input_path));
        }
        if self.segment_duration < 2 || self.segment_duration > 60 {
            return Err("Segment duration must be between 2 and 60 seconds".to_string());
        }
        Ok(())
    }
}

/// 构建单码率 HLS 切片命令
pub fn build_hls_command(config: &HlsSliceConfig) -> Result<Command, String> {
    config.validate()?;

    let master_playlist = config.output_dir.join(format!("{}.m3u8", config.output_prefix));
    let segment_pattern = config
        .output_dir
        .join(format!("{}_%03d.ts", config.output_prefix));

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");
    cmd.arg("-i").arg(&config.input_path);

    // HLS 编码参数
    cmd.arg("-c:v").arg("libx264");
    cmd.arg("-c:a").arg("aac");
    cmd.arg("-b:a").arg("128k");

    // HLS 分段参数
    cmd.arg("-f").arg("hls");
    cmd.arg("-hls_time").arg(config.segment_duration.to_string());
    cmd.arg("-hls_list_size").arg(config.playlist_size.to_string());
    cmd.arg("-hls_segment_filename").arg(&segment_pattern);

    // 强制关键帧对齐，确保分段在关键帧处切割
    cmd.arg("-force_key_frames")
        .arg(format!("expr:gte(t,n_forced*{})", config.segment_duration));
    cmd.arg("-sc_threshold").arg("0"); // 禁用场景检测

    cmd.arg(&master_playlist);
    Ok(cmd)
}

/// 构建多码率 HLS 切片命令（包含 master playlist）
pub fn build_multi_bitrate_hls(
    config: &HlsSliceConfig,
) -> Result<Command, String> {
    config.validate()?;

    if config.bitrate_ladder.is_empty() {
        return Err("Bitrate ladder cannot be empty for multi-bitrate HLS".to_string());
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");
    cmd.arg("-i").arg(&config.input_path);

    // 为每个码率变体构建输出映射
    let mut filter_complex = String::new();
    let mut maps = Vec::new();

    for (i, variant) in config.bitrate_ladder.iter().enumerate() {
        let height = if variant.height > 0 {
            variant.height.to_string()
        } else {
            // 自动计算高度（16:9 比例）
            (variant.width * 9 / 16).to_string()
        };

        // 缩放滤镜
        filter_complex.push_str(&format!(
            "[v:0]split={}[v{}];[v{}]scale={}:{}[v{}out];",
            config.bitrate_ladder.len(),
            i + 1,
            i + 1,
            variant.width,
            height,
            i + 1,
        ));

        // 变体输出配置
        let variant_playlist = config
            .output_dir
            .join(format!("{}_{}p.m3u8", config.output_prefix, height));
        let variant_segments = config
            .output_dir
            .join(format!("{}_{}p_%03d.ts", config.output_prefix, height));

        cmd.arg("-map").arg(format!("[v{}out]", i + 1));
        cmd.arg("-map").arg("a:0");
        cmd.arg(format!("-c:v:{}", i)).arg("libx264");
        cmd.arg(format!("-b:v:{}", i)).arg(&variant.video_bitrate);
        cmd.arg(format!("-maxrate:{}", i)).arg(&variant.max_bitrate);
        cmd.arg(format!("-bufsize:{}", i))
            .arg(format!("{}", variant.bandwidth / 4));
        cmd.arg(format!("-c:a:{}", i)).arg("aac");
        cmd.arg(format!("-b:a:{}", i)).arg(&variant.audio_bitrate);

        cmd.arg("-f").arg("hls");
        cmd.arg("-hls_time").arg(config.segment_duration.to_string());
        cmd.arg("-hls_list_size").arg(config.playlist_size.to_string());
        cmd.arg("-hls_segment_filename").arg(variant_segments);
        cmd.arg(variant_playlist);

        maps.push(format!("v:{},a:{}", i, i));
    }

    if !filter_complex.is_empty() {
        cmd.arg("-filter_complex").arg(&filter_complex);
    }

    // 生成 master playlist
    let master_playlist = config
        .output_dir
        .join(format!("{}.m3u8", config.output_prefix));
    cmd.arg("-master_pl_name").arg(&master_playlist);

    Ok(cmd)
}

/// 构建 DASH 切片命令
pub fn build_dash_command(config: &HlsSliceConfig) -> Result<Command, String> {
    config.validate()?;

    let mpd_path = config
        .output_dir
        .join(format!("{}.mpd", config.output_prefix));

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");
    cmd.arg("-i").arg(&config.input_path);

    cmd.arg("-c:v").arg("libx264");
    cmd.arg("-c:a").arg("aac");
    cmd.arg("-b:a").arg("128k");

    cmd.arg("-f").arg("dash");
    cmd.arg("-seg_duration").arg(config.segment_duration.to_string());
    cmd.arg("-use_timeline").arg("1");
    cmd.arg("-use_template").arg("1");
    cmd.arg("-init_seg_name")
        .arg(format!("{}_init_$RepresentationID$.m4s", config.output_prefix));
    cmd.arg("-media_seg_name")
        .arg(format!("{}_$RepresentationID$_$Number$.m4s", config.output_prefix));

    cmd.arg(&mpd_path);
    Ok(cmd)
}
