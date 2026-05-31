use crate::channels::messages::UiMessages;
use crate::core::player::mpv::{MpvCommand, PlaybackState};
use crate::core::player::window::MpvPlayer;
use egui_inbox::UiInboxSender;

/// MPV 播放器服务（封装线程通信）
pub struct MpvService {
    player: MpvPlayer,
    ui_sender: UiInboxSender<UiMessages>,
}

impl MpvService {
    /// 创建新的 MPV 服务
    pub fn new(ui_sender: UiInboxSender<UiMessages>) -> Self {
        Self {
            player: MpvPlayer::new(),
            ui_sender,
        }
    }

    /// 启动 MPV 并加载文件
    pub fn launch_and_run(&mut self, file_path: &str) -> Result<(), String> {
        let ipc_socket = get_ipc_socket_path();

        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(&ipc_socket);
        }

        self.player.launch(&ipc_socket)?;
        self.player.load_and_play(file_path)?;

        let _ = self.ui_sender.send(UiMessages::PlaybackEvent {
            event: "started".into(),
            data: Some(file_path.to_string()),
        });

        Ok(())
    }

    /// 轮询 MPV 事件
    pub fn poll(&mut self) {
        let events = self.player.poll_events();
        for _event_str in events {
            let _ = self.ui_sender.send(UiMessages::PlaybackStateUpdate {
                state: self.player.state.clone(),
            });
        }
    }

    pub fn send(&mut self, cmd: MpvCommand) -> Result<(), String> {
        self.player.send_command(&cmd)
    }

    pub fn toggle_playback(&mut self) -> Result<(), String> {
        self.player.toggle_pause()
    }

    pub fn seek(&mut self, secs: f64) -> Result<(), String> {
        self.player.seek(secs)
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<(), String> {
        self.player.set_volume(volume)
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.player.stop()
    }

    pub fn shutdown(&mut self) {
        self.player.shutdown();
    }

    pub fn state(&self) -> &PlaybackState {
        &self.player.state
    }

    pub fn is_running(&self) -> bool {
        self.player.is_running
    }
}

fn get_ipc_socket_path() -> String {
    #[cfg(target_os = "windows")]
    {
        r"\\.\pipe\freebird-mpv-socket".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let temp = std::env::temp_dir();
        temp.join("freebird-mpv-socket")
            .to_string_lossy()
            .to_string()
    }
}
