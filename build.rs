fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/freebird-format-converter.ico");
    res.set("FileDescription", "A convenient media file format converter");
    res.set("ProductName", "freebird format converter");
    res.set("CompanyName", "＞﹏＜");
    res.set("LegalCopyright", "copyright © 2026 freebird");
    res.set("FileVersion", "0.1");
    res.set("ProductVersion", "0.1");
    res.compile().unwrap();
}
