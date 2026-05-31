use std::path::PathBuf;
use std::process::Command;

// ──────────────────────────────────────────────
// 滤镜类型定义
// ──────────────────────────────────────────────

/// 滤镜参数类型
#[derive(Debug, Clone)]
pub enum FilterParam {
    Int { name: &'static str, value: i32, min: i32, max: i32 },
    Float { name: &'static str, value: f64, min: f64, max: f64 },
    StringParam { name: &'static str, value: String },
    Bool { name: &'static str, value: bool },
}

/// 滤镜定义
#[derive(Debug, Clone)]
pub enum Filter {
    /// 缩放滤镜: scale=width:height
    Scale { width: i32, height: i32 },
    /// 裁剪滤镜: crop=w:h:x:y
    Crop { width: i32, height: i32, x: i32, y: i32 },
    /// 旋转滤镜
    Rotate { angle: f64 },
    /// 水印叠加: overlay=x:y
    Watermark { image_path: PathBuf, x: i32, y: i32, opacity: f64 },
    /// 去隔行
    Deinterlace,
    /// 色彩调整: eq=brightness:contrast:saturation
    ColorAdjust { brightness: f64, contrast: f64, saturation: f64 },
    /// 模糊
    Blur { sigma: f64 },
    /// 锐化
    Sharpen { amount: f64 },
    /// 翻转: hflip / vflip
    Flip { horizontal: bool, vertical: bool },
    /// 自定义滤镜字符串
    Custom(String),
}

impl Filter {
    /// 将滤镜转换为 FFmpeg 滤镜字符串
    pub fn to_ffmpeg_filter(&self) -> String {
        match self {
            Self::Scale { width, height } => {
                if *height > 0 {
                    format!("scale={}:{}", width, height)
                } else {
                    format!("scale={}:-1", width) // 保持宽高比
                }
            }
            Self::Crop { width, height, x, y } => {
                format!("crop={}:{}:{}:{}", width, height, x, y)
            }
            Self::Rotate { angle } => {
                // 将角度转换为弧度
                let rad = angle.to_radians();
                format!("rotate={:.4}", rad)
            }
            Self::Watermark { image_path: _, x, y, opacity } => {
                // 水印需要两输入流：主视频 + 水印图
                // 返回占位，实际由 build_filter_complex_command 构建
                format!(
                    "[1:v]format=rgba,colorchannelmixer=aa={}[wm];[0:v][wm]overlay={}:{}",
                    opacity, x, y
                )
            }
            Self::Deinterlace => "yadif".to_string(),
            Self::ColorAdjust { brightness, contrast, saturation } => {
                format!(
                    "eq=brightness={}:contrast={}:saturation={}",
                    brightness, contrast, saturation
                )
            }
            Self::Blur { sigma } => {
                format!("gblur=sigma={}", sigma)
            }
            Self::Sharpen { amount } => {
                format!("unsharp=5:5:{:.1}:5:5:0.0", amount)
            }
            Self::Flip { horizontal, vertical } => {
                let mut filters = Vec::new();
                if *horizontal {
                    filters.push("hflip");
                }
                if *vertical {
                    filters.push("vflip");
                }
                filters.join(",")
            }
            Self::Custom(s) => s.clone(),
        }
    }
}

// ──────────────────────────────────────────────
// 滤镜链
// ──────────────────────────────────────────────

/// 滤镜链：按顺序应用多个滤镜
#[derive(Debug, Clone, Default)]
pub struct FilterChain {
    filters: Vec<Filter>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self { filters: Vec::new() }
    }

    /// 添加滤镜
    pub fn add(&mut self, filter: Filter) {
        self.filters.push(filter);
    }

    /// 移除指定索引的滤镜
    pub fn remove(&mut self, index: usize) -> Option<Filter> {
        if index < self.filters.len() {
            Some(self.filters.remove(index))
        } else {
            None
        }
    }

    /// 移动滤镜位置
    pub fn reorder(&mut self, from: usize, to: usize) -> bool {
        if from < self.filters.len() && to < self.filters.len() {
            let filter = self.filters.remove(from);
            self.filters.insert(to, filter);
            true
        } else {
            false
        }
    }

    /// 滤镜数量
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// 生成 FFmpeg -vf 参数值
    pub fn to_vf_string(&self) -> String {
        self.filters
            .iter()
            .map(|f| f.to_ffmpeg_filter())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// 获取所有滤镜的引用
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    /// 获取所有滤镜的可变引用
    pub fn filters_mut(&mut self) -> &mut Vec<Filter> {
        &mut self.filters
    }
}

// ──────────────────────────────────────────────
// 渲染请求
// ──────────────────────────────────────────────

/// 滤镜渲染请求
#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub filter_chain: FilterChain,
    /// 视频编码器
    pub video_encoder: Option<String>,
    /// CRF 质量
    pub crf: Option<u32>,
    /// 音频编码器（默认 copy）
    pub audio_encoder: Option<String>,
}

impl RenderRequest {
    pub fn validate(&self) -> Result<(), String> {
        if !self.input_path.exists() {
            return Err(format!("Input file not found: {:?}", self.input_path));
        }
        if self.filter_chain.is_empty() {
            return Err("Filter chain cannot be empty".to_string());
        }
        Ok(())
    }
}

/// 构建简单滤镜渲染命令（-vf 单输入）
pub fn build_simple_filter_command(req: &RenderRequest) -> Result<Command, String> {
    req.validate()?;

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");
    cmd.arg("-i").arg(&req.input_path);

    // 应用滤镜链
    let vf = req.filter_chain.to_vf_string();
    cmd.arg("-vf").arg(&vf);

    // 编码器设置
    if let Some(ref enc) = req.video_encoder {
        cmd.arg("-c:v").arg(enc);
    } else {
        cmd.arg("-c:v").arg("libx264");
    }
    if let Some(crf) = req.crf {
        cmd.arg("-crf").arg(crf.to_string());
    }
    if let Some(ref enc) = req.audio_encoder {
        cmd.arg("-c:a").arg(enc);
    } else {
        cmd.arg("-c:a").arg("copy");
    }

    cmd.arg(&req.output_path);
    Ok(cmd)
}

/// 构建复杂滤镜命令（-filter_complex，支持多输入如加水印）
pub fn build_filter_complex_command(
    req: &RenderRequest,
    extra_inputs: &[(PathBuf, String)], // (文件路径, 流标签如 "wm")
) -> Result<Command, String> {
    req.validate()?;

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-hide_banner");

    // 主输入
    cmd.arg("-i").arg(&req.input_path);

    // 额外输入（水印图等）
    for (path, _label) in extra_inputs {
        cmd.arg("-i").arg(path);
    }

    // 构建 filter_complex 字符串
    let complex = req.filter_chain.to_vf_string();
    cmd.arg("-filter_complex").arg(&complex);

    // 编码器
    if let Some(ref enc) = req.video_encoder {
        cmd.arg("-c:v").arg(enc);
    } else {
        cmd.arg("-c:v").arg("libx264");
    }
    if let Some(crf) = req.crf {
        cmd.arg("-crf").arg(crf.to_string());
    }
    if let Some(ref enc) = req.audio_encoder {
        cmd.arg("-c:a").arg(enc);
    } else {
        cmd.arg("-c:a").arg("copy");
    }

    cmd.arg(&req.output_path);
    Ok(cmd)
}

// ──────────────────────────────────────────────
// 预设滤镜
// ──────────────────────────────────────────────

/// 创建缩放至指定分辨率的滤镜链
pub fn preset_resize(width: i32, height: i32) -> FilterChain {
    let mut chain = FilterChain::new();
    chain.add(Filter::Scale { width, height });
    chain
}

/// 创建 16:9 裁剪滤镜链
pub fn preset_crop_16_9() -> FilterChain {
    let mut chain = FilterChain::new();
    // crop=in_w:in_w*9/16 从中心裁剪 16:9
    chain.add(Filter::Custom(
        "crop=in_w:in_w*9/16".to_string()
    ));
    chain
}

/// 创建色彩校正预设
pub fn preset_color_correction(brightness: f64, contrast: f64, saturation: f64) -> FilterChain {
    let mut chain = FilterChain::new();
    chain.add(Filter::ColorAdjust { brightness, contrast, saturation });
    chain
}

/// 创建画面增强预设（轻微锐化 + 色彩增强）
pub fn preset_enhance() -> FilterChain {
    let mut chain = FilterChain::new();
    chain.add(Filter::Sharpen { amount: 0.5 });
    chain.add(Filter::ColorAdjust {
        brightness: 0.0,
        contrast: 1.05,
        saturation: 1.1,
    });
    chain
}

/// 创建去噪 + 去隔行预设
pub fn preset_restore() -> FilterChain {
    let mut chain = FilterChain::new();
    chain.add(Filter::Deinterlace);
    chain.add(Filter::Blur { sigma: 0.5 });
    chain
}
