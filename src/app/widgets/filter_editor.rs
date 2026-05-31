use egui::{Color32, Ui};
use crate::core::processor::renderer::{Filter, FilterChain};

/// 滤镜链编辑器组件
pub struct FilterEditor {
    /// 拖拽中的滤镜索引
    drag_source: Option<usize>,
}

impl FilterEditor {
    pub fn new() -> Self {
        Self { drag_source: None }
    }

    /// 渲染滤镜链编辑器
    pub fn render(&mut self, ui: &mut Ui, chain: &mut FilterChain) {
        ui.horizontal(|ui| {
            ui.heading("Filter Chain");
            ui.add_space(8.0);
            if ui.button("+ Add Filter").clicked() {
                // 默认添加缩放滤镜作为起点
                chain.add(Filter::Scale {
                    width: 1920,
                    height: 0, // 保持宽高比
                });
            }
            if ui.button("Reset").clicked() {
                *chain = FilterChain::new();
            }
        });

        ui.add_space(4.0);

        if chain.is_empty() {
            ui.label("No filters. Click '+ Add Filter' to start.");
            return;
        }

        // 渲染每个滤镜的编辑卡片
        let mut remove_index = None;
        let mut move_up = None;
        let mut move_down = None;

        for (i, filter) in chain.filters().iter().enumerate() {
            ui.group(|ui| {
                render_filter_card(ui, i, filter, &mut remove_index, &mut move_up, &mut move_down);
            });
            ui.add_space(2.0);
        }

        // 处理操作
        if let Some(idx) = remove_index {
            chain.remove(idx);
        }
        if let Some(idx) = move_up {
            if idx > 0 {
                chain.reorder(idx, idx - 1);
            }
        }
        if let Some(idx) = move_down {
            if idx < chain.len() - 1 {
                chain.reorder(idx, idx + 1);
            }
        }
    }
}

fn render_filter_card(
    ui: &mut Ui,
    index: usize,
    filter: &Filter,
    remove_index: &mut Option<usize>,
    move_up: &mut Option<usize>,
    move_down: &mut Option<usize>,
) {
    egui::Frame::default()
        .inner_margin(egui::Margin::same(4))
        .fill(Color32::from_rgb(40, 40, 45))
        .corner_radius(4)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // 滤镜编号和类型
                ui.label(format!("#{}", index + 1));
                ui.colored_label(Color32::from_rgb(0, 180, 216), filter_name(filter));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("X").clicked() {
                        *remove_index = Some(index);
                    }
                    if ui.button("v").clicked() {
                        *move_down = Some(index);
                    }
                    if ui.button("^").clicked() {
                        *move_up = Some(index);
                    }
                });
            });

            // 滤镜参数（简化版）
            ui.add_space(2.0);
            match filter {
                Filter::Scale { width, height } => {
                    ui.label(format!("  Size: {}x{}", width, if *height > 0 { height.to_string() } else { "auto".into() }));
                }
                Filter::Crop { width, height, x, y } => {
                    ui.label(format!("  Crop: {}x{} at ({},{})", width, height, x, y));
                }
                Filter::Rotate { angle } => {
                    ui.label(format!("  Angle: {:.1}°", angle));
                }
                Filter::Watermark { image_path: _, x, y, opacity } => {
                    ui.label(format!("  Watermark at ({},{}) opacity {:.0}%", x, y, opacity * 100.0));
                }
                Filter::Deinterlace => {
                    ui.label("  Mode: yadif");
                }
                Filter::ColorAdjust { brightness, contrast, saturation } => {
                    ui.label(format!(
                        "  Brightness: {:.2} | Contrast: {:.2} | Saturation: {:.2}",
                        brightness, contrast, saturation
                    ));
                }
                Filter::Blur { sigma } => {
                    ui.label(format!("  Sigma: {:.1}", sigma));
                }
                Filter::Sharpen { amount } => {
                    ui.label(format!("  Amount: {:.1}", amount));
                }
                Filter::Flip { horizontal, vertical } => {
                    let dir = match (horizontal, vertical) {
                        (true, true) => "Both",
                        (true, false) => "Horizontal",
                        (false, true) => "Vertical",
                        _ => "None",
                    };
                    ui.label(format!("  Flip: {}", dir));
                }
                Filter::Custom(s) => {
                    ui.label(format!("  Custom: {}", s));
                }
            }
        });
}

fn filter_name(filter: &Filter) -> &'static str {
    match filter {
        Filter::Scale { .. } => "Scale",
        Filter::Crop { .. } => "Crop",
        Filter::Rotate { .. } => "Rotate",
        Filter::Watermark { .. } => "Watermark",
        Filter::Deinterlace => "Deinterlace",
        Filter::ColorAdjust { .. } => "Color Adjust",
        Filter::Blur { .. } => "Blur",
        Filter::Sharpen { .. } => "Sharpen",
        Filter::Flip { .. } => "Flip",
        Filter::Custom(_) => "Custom",
    }
}

impl Default for FilterEditor {
    fn default() -> Self {
        Self::new()
    }
}
