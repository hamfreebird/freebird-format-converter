fn main() {
    let mut res = winresource::WindowsResource::new();
    // 这里 "my-icon.ico" 应替换为你的图标文件名
    res.set_icon("assets/freebird-format-converter.ico");
    // 设置文件属性
    res.set("FileDescription", "A convenient media file format converter, \
    based on ffmpeg, almost supports all formats of video/audio/images");
    res.set("ProductName", "freebird format converter");
    res.set("CompanyName", "＞﹏＜");
    res.set("LegalCopyright", "copyright © 2026 freebird");
    res.set("FileVersion", "0.1");
    res.set("ProductVersion", "0.1");
    res.compile().unwrap();
}
