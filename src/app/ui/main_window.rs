use crate::channels::messages::UiMessages;
use crate::core::{EncoderInfo, FormatInfo, PixelFormatInfo};
use crate::truncate_str;
use egui::Ui;
use egui_inbox::UiInboxSender;
use std::path::PathBuf;

pub fn render_main_window(
    ui: &mut Ui,
    picker_id: Option<u8>,
    current_film_path: &Option<PathBuf>,
    current_folder_path: &Option<PathBuf>,
    sender: UiInboxSender<UiMessages>,
    is_running: bool,
    encoder_info: Vec<EncoderInfo>,
    format_info: Vec<FormatInfo>,
    pixel_format_info: Vec<PixelFormatInfo>,
    encoder_name: Vec<String>,
    format_name: Vec<String>,
    pixel_format_names: Vec<String>,
    selected_encoder: &mut String,
    selected_format: &mut String,
    selected_pixel_format: &mut String,
    _is_video: &mut bool,
    _is_audio: &mut bool,
    _is_subtitle: &mut bool,
    error_message: Option<String>,
    bitrate: &mut String,                  // 目标比特率
    constant_rate_factor: &mut String,     // 恒定质量模式 0-51
    coding_default: &mut String,       // 编码预设
    gop: &mut String,
    output_lines: &mut Vec<String>,) {
    egui::Frame::default()
        .inner_margin(egui::Margin::same(10i8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Log: ");
                // 显示运行状态
                ui.label(error_message
                    .as_deref()                     // Option<&str>
                    .unwrap_or("As right as rain~  ヾ(≧▽≦*)o")           // &str
                    .to_string());
            });

            ui.separator();

            render_top_area(ui,
                            picker_id,
                            current_film_path,
                            current_folder_path,
                            sender.clone(),
                            is_running);

            ui.separator();

            render_mid_area(ui,
                            encoder_info,
                            format_info,
                            pixel_format_info,
                            encoder_name,
                            format_name,
                            pixel_format_names,
                            selected_encoder,
                            selected_format,
                            selected_pixel_format,
                            _is_video,
                            _is_audio,
                            _is_subtitle);

            ui.separator();

            // 输入详细参数
            ui.horizontal(|ui| {
                ui.label("Bitrate:");
                ui.add(egui::TextEdit::singleline(bitrate).desired_width(50.0));
                ui.add_space(10.0);
                ui.label("crf:");
                ui.add(egui::TextEdit::singleline(constant_rate_factor).desired_width(50.0));
                ui.add_space(10.0);
                ui.label("Preset:");
                ui.add(egui::TextEdit::singleline(coding_default).desired_width(50.0));
                ui.add_space(10.0);
                ui.label("GOP:");
                ui.add(egui::TextEdit::singleline(gop).desired_width(50.0));
                ui.add_space(10.0);
            });

            ui.separator();

            // ---------- 底部区域：多行文本输出框 ----------
            // 输出显示区域
            ui.label("Output:");
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut output_lines.join("\n"))
                            .desired_width(f32::INFINITY)
                            .desired_rows(20)
                            .interactive(false) // 只读
                            .font(egui::TextStyle::Monospace),
                    );
                });

        });
}

fn render_top_area(
    ui: &mut Ui,
    picker_id: Option<u8>,
    current_film_path: &Option<PathBuf>,
    current_folder_path: &Option<PathBuf>,
    sender: UiInboxSender<UiMessages>,
    is_running: bool) {
    // ---------- 顶部区域：两个文件选择 + 按钮 ----------
    ui.horizontal(|ui| {
        // 预留一些空间给左侧的输入框和按钮
        let available_width = ui.available_width();
        let picker_width = (available_width - 100.0) / 2.0; // 粗略计算，100.0 预留给 Run/Stop 按钮

        // === 第一个文件选择 ===
        ui.horizontal(|ui| {
            ui.set_width(picker_width);
            // 文件路径显示标签
            let path1_text = current_film_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Select the files to be converted");

            // 使用 add_sized 限制标签的宽度，防止长文件名破坏布局
            ui.add_sized(
                [picker_width - 70.0, ui.spacing().interact_size.y],
                egui::Label::new(path1_text).truncate(),
            );

            // 浏览按钮
            if ui.button("Browse...").clicked() {
                sender.send(UiMessages::PickFile(picker_id)).ok();
            }
        });

        ui.add_space(10.0);

        // === 第一个文件夹选择 ===
        ui.horizontal(|ui| {
            ui.set_width(picker_width);
            let path2_text = current_folder_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Select the folder as the output directory");

            ui.add_sized(
                [picker_width - 70.0, ui.spacing().interact_size.y],
                egui::Label::new(path2_text).truncate(),
            );

            if ui.button("Browse...").clicked() {
                sender.send(UiMessages::PickFolder(picker_id)).ok();
            }
        });

        // 弹性空间，将运行/停止按钮推到最右边
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button_enabled = !is_running;
            if ui
                .add_enabled(button_enabled, egui::Button::new("Run"))
                .clicked()
            {
                sender.send(UiMessages::StartFFMPEG).ok();
            }
            if is_running {
                if ui.button("Stop").clicked() {
                    sender.send(UiMessages::StopFFMPEG).ok();
                }
            }
        });
    });
}

fn render_mid_area(
    ui: &mut Ui,
    encoder_info: Vec<EncoderInfo>,
    format_info: Vec<FormatInfo>,
    pixel_format_info: Vec<PixelFormatInfo>,
    encoder_name: Vec<String>,
    format_name: Vec<String>,
    pixel_format_names: Vec<String>,
    selected_encoder: &mut String,
    selected_format: &mut String,
    selected_pixel_format: &mut String,
    mut _is_video: &mut bool,
    mut _is_audio: &mut bool,
    mut _is_subtitle: &mut bool,
) {
    // ---------- 中间区域 ----------
    ui.horizontal(|ui| {
        ui.label("Encoders:");
        let combo_encoder_response = egui::ComboBox::from_id_salt("encoder")
            .selected_text(truncate_str(&selected_encoder, 100))
            .width(150.0)
            .show_ui(ui, |ui| {
                for (index, name) in encoder_name.iter().enumerate() {
                    let display_text = truncate_str(name, 100);
                    let mut is_selected = *selected_encoder == *name;
                    // 使用 selectable_label 获得 Response
                    let response = ui.selectable_label(is_selected, display_text);
                    // 始终显示完整提示，显示对应的description
                    if let Some(encoder_info) = encoder_info.get(index) {
                        let mut type_info: String = "Unknown".to_string();
                        if (encoder_info.is_video == true) & (encoder_info.is_audio == true)
                        {type_info = "Video/Audio".to_string()}
                        else if (encoder_info.is_video == true) & (encoder_info.is_audio == false)
                        {type_info = "Video".to_string()}
                        else if (encoder_info.is_video == false) & (encoder_info.is_audio == true)
                        {type_info = "Audio".to_string()};
                        if encoder_info.is_subtitle == true {type_info = "Subtitle".to_string()};
                        let hover_text = format!(
                            "Type: {}\nDescription: {}\nFrame-level multithreading: {}\n\
                                            Slice-level multithreading: {}\nCodec is experimental: {}\n",
                            type_info,
                            &encoder_info.description,
                            &encoder_info.is_frame_multithreading,
                            &encoder_info.is_slice_multithreading,
                            &encoder_info.is_experimental,
                        );
                        response.clone().on_hover_text(hover_text);
                    }
                    // 手动处理点击事件
                    if response.clicked() {
                        *selected_encoder = name.clone();
                        *_is_video = encoder_info.get(index).unwrap().is_video;
                        *_is_audio = encoder_info.get(index).unwrap().is_audio;
                        *_is_subtitle = encoder_info.get(index).unwrap().is_subtitle;
                        ui.checkbox(&mut is_selected, ());
                    }
                }
            });
        // 为当前选中的编码器添加悬停提示，显示对应的description
        if !selected_encoder.is_empty() {
            if let Some(encoder_info) = encoder_info
                .iter()
                .find(|e| e.name == *selected_encoder)
            {
                let mut type_info: String = "Unknown".to_string();
                if (encoder_info.is_video == true) & (encoder_info.is_audio == true)
                {type_info = "Video/Audio".to_string()}
                else if (encoder_info.is_video == true) & (encoder_info.is_audio == false)
                {type_info = "Video".to_string()}
                else if (encoder_info.is_video == false) & (encoder_info.is_audio == true)
                {type_info = "Audio".to_string()};
                if encoder_info.is_subtitle == true {type_info = "Subtitle".to_string()};
                let hover_text = format!(
                    "Type: {}\nDescription: {}\nFrame-level multithreading: {}\n\
                                    Slice-level multithreading: {}\nCodec is experimental: {}\n",
                    type_info,
                    &encoder_info.description,
                    &encoder_info.is_frame_multithreading,
                    &encoder_info.is_slice_multithreading,
                    &encoder_info.is_experimental,
                );
                combo_encoder_response.response.on_hover_text(hover_text);
            } else if *selected_encoder
                != truncate_str(&selected_encoder, 100)
            {
                combo_encoder_response.response.on_hover_text(&*selected_encoder);
            }
        }

        ui.add_space(5.0);
        ui.label("Layouts:");
        let combo_format_response = egui::ComboBox::from_id_salt("format")
            .selected_text(truncate_str(&selected_format, 100))
            .width(200.0)
            .show_ui(ui, |ui| {
                for (index, name) in format_name.iter().enumerate() {
                    let display_text = truncate_str(name, 100);
                    let mut is_selected = *selected_format == *name;
                    // 使用 selectable_label 获得 Response
                    let response = ui.selectable_label(is_selected, display_text);
                    // 始终显示完整提示，显示对应的description
                    if let Some(format_info) = format_info.get(index) {
                        let hover_text = format!(
                            "Description: {}\nCan be read/unsealed as input: {}\n\
                                            Can be written/encapsulated as output: {}",
                            &format_info.description,
                            &format_info.can_mux,
                            &format_info.can_demux
                        );
                        response.clone().on_hover_text(hover_text);
                    }
                    // 手动处理点击事件
                    if response.clicked() {
                        *selected_format = name.clone();
                        ui.checkbox(&mut is_selected, ());
                    }
                }
            });
        // 为当前选中的格式添加悬停提示，显示对应的description
        if !selected_format.is_empty() {
            if let Some(format_info) = format_info
                .iter()
                .find(|f| f.name == *selected_format)
            {
                let hover_text = format!(
                    "Description: {}\nCan be read/unsealed as input: {}\n\
                                    Can be written/encapsulated as output: {}",
                    &format_info.description,
                    &format_info.can_mux,
                    &format_info.can_demux
                );
                combo_format_response.response.on_hover_text(hover_text);
            } else if *selected_format
                != truncate_str(&selected_format, 100)
            {
                combo_format_response.response.on_hover_text(&*selected_format);
            }
        }

        ui.add_space(5.0);
        ui.label("PixFmts:");
        let combo_pixel_response = egui::ComboBox::from_id_salt("pixel")
            .selected_text(&*selected_pixel_format)
            .width(100.0)
            .show_ui(ui, |ui| {
                for (index, name) in pixel_format_names.iter().enumerate() {
                    let response = ui.selectable_value(
                        selected_pixel_format,
                        name.clone(),
                        name,
                    );
                    // 始终显示完整提示，显示详细信息
                    if let Some(pixel_info) = pixel_format_info.get(index) {
                        let hover_text = format!(
                            "Name: {}\nInput: {}\nOutput: {}\nNumber per pixel: {}",
                            pixel_info.name,
                            if pixel_info.input_ok { "True" } else { "False" },
                            if pixel_info.output_ok { "True" } else { "False" },
                            pixel_info.bits_per_pixel
                        );
                        response.on_hover_text(hover_text);
                    }
                }
            });
        // 为当前选中的像素格式添加悬停提示
        if !selected_pixel_format.is_empty() {
            if let Some(pixel_info) = pixel_format_info
                .iter()
                .find(|p| p.name == *selected_pixel_format)
            {
                let hover_text = format!(
                    "Name: {}\nInput: {}\nOutput: {}\nNumber per pixel: {}",
                    pixel_info.name,
                    if pixel_info.input_ok { "True" } else { "False" },
                    if pixel_info.output_ok { "True" } else { "False" },
                    pixel_info.bits_per_pixel
                );
                combo_pixel_response.response.on_hover_text(hover_text);
            }
        }
    });
}
