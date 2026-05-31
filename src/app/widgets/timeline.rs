use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// 剪辑时间轴组件
pub struct ClipTimeline {
    pub duration_secs: f64,
    pub in_point_secs: f64,
    pub out_point_secs: f64,
    pub playhead_secs: Option<f64>,
    dragging_in: bool,
    dragging_out: bool,
}

impl ClipTimeline {
    pub fn new(duration_secs: f64) -> Self {
        Self {
            duration_secs,
            in_point_secs: 0.0,
            out_point_secs: duration_secs,
            playhead_secs: None,
            dragging_in: false,
            dragging_out: false,
        }
    }

    pub fn selection_duration(&self) -> f64 {
        self.out_point_secs - self.in_point_secs
    }

    /// 渲染时间轴
    pub fn render(&mut self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(ui.available_width(), 60.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let painter = ui.painter();
        let corner = egui::CornerRadius::same(4);

        // 背景
        painter.rect_filled(rect, corner, Color32::from_rgb(30, 30, 30));
        painter.rect_stroke(rect, corner, Stroke::new(1.0, Color32::from_rgb(60, 60, 60)), egui::StrokeKind::Inside);

        let track_y = rect.center().y;
        let track_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + 10.0, track_y - 10.0),
            Vec2::new(rect.width() - 20.0, 20.0),
        );

        // 轨道背景
        painter.rect_filled(track_rect, egui::CornerRadius::same(2), Color32::from_rgb(50, 50, 50));

        let track_w = track_rect.width();
        let dur = if self.duration_secs > 0.0 { self.duration_secs } else { 1.0 };

        let in_x = track_rect.min.x + (self.in_point_secs / dur) as f32 * track_w;
        let out_x = track_rect.min.x + (self.out_point_secs / dur) as f32 * track_w;

        // 选区高亮
        let selection_rect = Rect::from_min_max(
            Pos2::new(in_x, track_rect.min.y),
            Pos2::new(out_x, track_rect.max.y),
        );
        painter.rect_filled(selection_rect, egui::CornerRadius::same(2), Color32::from_rgb(0, 180, 216));

        // 入点/出点手柄
        let handle_color = Color32::WHITE;
        let hw = 4.0;
        let hh = track_rect.height() + 6.0;

        painter.rect_filled(
            Rect::from_center_size(Pos2::new(in_x, track_y), Vec2::new(hw, hh)),
            egui::CornerRadius::same(2),
            handle_color,
        );
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(out_x, track_y), Vec2::new(hw, hh)),
            egui::CornerRadius::same(2),
            handle_color,
        );

        // 播放头
        if let Some(ph) = self.playhead_secs {
            let ph_x = track_rect.min.x + (ph / dur) as f32 * track_w;
            let ph_x = ph_x.clamp(track_rect.min.x, track_rect.max.x);
            painter.line_segment(
                [Pos2::new(ph_x, track_rect.min.y - 6.0), Pos2::new(ph_x, track_rect.max.y + 6.0)],
                Stroke::new(2.0, Color32::from_rgb(255, 80, 80)),
            );
        }

        // 时间标签 — 使用 ui 绘制避免 painter.text API 问题
        let font_id = egui::FontId::proportional(9.0);
        // 入点
        ui.put(
            Rect::from_center_size(Pos2::new(in_x, track_rect.min.y - 10.0), Vec2::new(50.0, 12.0)),
            egui::Label::new(egui::RichText::new(format_time(self.in_point_secs)).font(font_id.clone()).color(Color32::from_rgb(200, 200, 200))),
        );
        // 出点
        ui.put(
            Rect::from_center_size(Pos2::new(out_x, track_rect.max.y + 10.0), Vec2::new(50.0, 12.0)),
            egui::Label::new(egui::RichText::new(format_time(self.out_point_secs)).font(font_id.clone()).color(Color32::from_rgb(200, 200, 200))),
        );
        // 总时长
        ui.put(
            Rect::from_min_size(Pos2::new(track_rect.max.x + 2.0, track_rect.max.y - 10.0), Vec2::new(50.0, 12.0)),
            egui::Label::new(egui::RichText::new(format_time(self.duration_secs)).font(font_id).color(Color32::from_rgb(150, 150, 150))),
        );

        // 交互
        if let Some(ptr) = response.hover_pos() {
            let near_in = (ptr.x - in_x).abs() < 10.0 || self.dragging_in;
            let near_out = (ptr.x - out_x).abs() < 10.0 || self.dragging_out;

            if near_in || near_out {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }

        if self.dragging_in || self.dragging_out {
            if let Some(ptr) = ui.ctx().pointer_latest_pos() {
                let t = ((ptr.x - track_rect.min.x) / track_w) as f64 * dur;
                let t = t.clamp(0.0, self.duration_secs);
                if self.dragging_in {
                    self.in_point_secs = t.min(self.out_point_secs - 0.1);
                }
                if self.dragging_out {
                    self.out_point_secs = t.max(self.in_point_secs + 0.1);
                }
            }
            if !ui.ctx().input(|i| i.pointer.primary_down()) {
                self.dragging_in = false;
                self.dragging_out = false;
            }
        }

        // 点击检测拖拽开始
        if response.clicked() {
            if let Some(ptr) = response.hover_pos() {
                let near_in = (ptr.x - in_x).abs() < 10.0;
                let near_out = (ptr.x - out_x).abs() < 10.0;
                if near_in {
                    self.dragging_in = true;
                } else if near_out {
                    self.dragging_out = true;
                }
            }
        }

        response
    }
}

fn format_time(secs: f64) -> String {
    let h = (secs / 3600.0) as u64;
    let m = ((secs % 3600.0) / 60.0) as u64;
    let s = secs % 60.0;
    if h > 0 {
        format!("{}:{:02}:{:04.1}", h, m, s)
    } else {
        format!("{}:{:04.1}", m, s)
    }
}
