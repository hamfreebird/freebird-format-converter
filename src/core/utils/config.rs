use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// 应用设置（可持久化到 JSON）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 最大并发任务数 (1-16)
    pub max_concurrent_tasks: u32,
    /// 默认输出目录
    pub default_output_dir: Option<String>,
    /// 自定义 FFmpeg 路径（None = 自动检测）
    pub ffmpeg_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            default_output_dir: None,
            ffmpeg_path: None,
        }
    }
}

impl AppSettings {
    /// 获取配置文件路径
    fn config_path() -> Option<PathBuf> {
        let mut path = dirs_next()?;
        path.push("freebird-format-converter");
        std::fs::create_dir_all(&path).ok()?;
        path.push("settings.json");
        Some(path)
    }

    /// 从磁盘加载设置，失败时返回默认值
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 保存设置到磁盘
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Cannot determine config directory")?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write settings: {}", e))?;
        Ok(())
    }
}

/// 获取用户配置目录（跨平台）
#[cfg(not(target_os = "windows"))]
fn dirs_next() -> Option<PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
}

#[cfg(target_os = "windows")]
fn dirs_next() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
}

pub(crate) fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 注册多个字体数据
    fonts.font_data.insert(
        "Ubuntu-Light".to_owned(),
        Arc::from(egui::FontData::from_static(include_bytes!("../../../assets/fonts/Ubuntu-Light.ttf"))),
    );
    fonts.font_data.insert(
        "simhei".to_owned(),
        Arc::from(egui::FontData::from_static(include_bytes!("../../../assets/fonts/simhei.ttf"))),
    );

    // 为 Proportional 家族设置优先级顺序
    let proportional_fonts = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional_fonts.clear();               // 清除默认字体
    proportional_fonts.push("Ubuntu-Light".to_owned());
    proportional_fonts.push("simhei".to_owned());

    // 为 Monospace 家族单独设置
    let monospace_fonts = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    monospace_fonts.clear();
    monospace_fonts.push("Ubuntu-Light".to_owned());
    monospace_fonts.push("simhei".to_owned());

    ctx.set_fonts(fonts);
}

pub(crate) fn load_icon_data() -> egui::IconData {
    // 将图像文件（如 favicon.png）作为字节数组嵌入
    let image_bytes = include_bytes!("../../../assets/freebird-format-converter.ico");
    // 使用 image 库解码图像
    let image = image::load_from_memory(image_bytes).expect("Failed to load icon");
    // 确保图像尺寸合适并转换为 RGBA
    let image = image.into_rgba8();

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    egui::IconData {
        rgba,
        width,
        height,
    }
}