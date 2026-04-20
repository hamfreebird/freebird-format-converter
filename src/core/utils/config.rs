use std::sync::Arc;

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