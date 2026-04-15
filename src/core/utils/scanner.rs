use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

// TODO:处理文件路径发送失败的情况

/// 触发文件选择对话框
pub(crate) fn pick_file(mut file_picker_tx: &mut Option<mpsc::SyncSender<PathBuf>>,
                        mut file_picker_rx: &mut Option<mpsc::Receiver<PathBuf>>,
                        mut active_picker: &mut Option<u8>,
                        picker_id: u8) {
    let (tx, rx) = mpsc::sync_channel(1);
    *file_picker_tx = Some(tx.clone());
    *file_picker_rx = Some(rx);
    *active_picker = Some(picker_id);

    // 在新线程中执行阻塞的文件对话框
    thread::spawn(move || {
        // rfd的阻塞API
        let file = rfd::FileDialog::new().pick_file();

        // 如果用户选择了文件，则通过通道发送路径
        if let Some(path) = file {
            let _ = tx.send(path);
        }
    });
}

/// 触发文件夹选择对话框
pub(crate) fn pick_folder(mut folder_picker_tx: &mut Option<mpsc::SyncSender<PathBuf>>,
                   mut folder_picker_rx: &mut Option<mpsc::Receiver<PathBuf>>,
                   mut active_folder_picker: &mut Option<u8>,
                   picker_id: u8) {
    let (tx, rx) = mpsc::sync_channel(1);
    *folder_picker_tx = Some(tx.clone());
    *folder_picker_rx = Some(rx);
    *active_folder_picker = Some(picker_id);

    // 在新线程中执行阻塞的文件夹对话框
    thread::spawn(move || {
        // 调用rfd
        let folder = rfd::FileDialog::new().pick_folder();

        // 如果用户选择了文件夹，则通过通道发送路径
        if let Some(path) = folder {
            let _ = tx.send(path);
        }
    });
}