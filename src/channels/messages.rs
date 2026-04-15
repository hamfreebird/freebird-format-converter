use std::path::PathBuf;

pub enum UiMessages {
    Increment,
    Decrement,
    LoadData(String), // 触发异步数据加载
    DataLoaded(Result<String, String>), // 异步操作的结果
    PickFile(Option<u8>),
    PickFolder(Option<u8>),
    FileSelected(Option<u8>, Option<PathBuf>),
    FolderSelected(Option<u8>, Option<PathBuf>),
    StartFFMPEG,
    StopFFMPEG,
}
