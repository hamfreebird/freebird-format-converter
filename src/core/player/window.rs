use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

use crate::core::player::mpv::{MpvCommand, PlaybackState};

/// MPV 播放器实例
pub struct MpvPlayer {
    /// MPV 子进程
    child: Option<Child>,
    /// 向 MPV 发送命令的写入端
    stdin_writer: Option<Box<dyn Write + Send>>,
    /// 接收 MPV 事件的通道
    event_receiver: Option<mpsc::Receiver<String>>,
    /// 当前播放状态
    pub state: PlaybackState,
    /// 是否成功启动
    pub is_running: bool,
}

impl MpvPlayer {
    /// 创建新的播放器实例（未启动）
    pub fn new() -> Self {
        Self {
            child: None,
            stdin_writer: None,
            event_receiver: None,
            state: PlaybackState::default(),
            is_running: false,
        }
    }

    /// 启动 MPV 进程
    /// `ipc_socket` 为 IPC socket 路径（Linux: /tmp/mpv-socket, Windows: \\\\.\\pipe\\mpv-pipe）
    pub fn launch(&mut self, ipc_socket: &str) -> Result<(), String> {
        if self.is_running {
            return Err("MPV is already running".to_string());
        }

        let mut cmd = Command::new("mpv");
        cmd.arg("--idle=yes")              // 启动后等待命令
            .arg("--keep-open=yes")         // 播放完毕后保持窗口
            .arg("--quiet")                 // 减少日志输出
            .arg("--no-terminal")           // 不使用终端
            .arg("--osc=no")                // 禁用内置 OSC（使用自定义 UI）
            .arg("--input-ipc-server")      // IPC 通信
            .arg(ipc_socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Windows: 隐藏控制台窗口
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to launch MPV: {}", e))?;

        let stdin = child.stdin.take()
            .ok_or("Failed to capture MPV stdin")?;
        let stdout = child.stdout.take()
            .ok_or("Failed to capture MPV stdout")?;

        // MPV 将 JSON 事件发送到 stdout
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.child = Some(child);
        self.stdin_writer = Some(Box::new(stdin));
        self.event_receiver = Some(rx);
        self.is_running = true;

        // 设置初始属性监听
        self.observe_playback_properties()?;

        Ok(())
    }

    /// 发送命令到 MPV
    pub fn send_command(&mut self, cmd: &MpvCommand) -> Result<(), String> {
        let writer = self.stdin_writer
            .as_mut()
            .ok_or("MPV not running")?;

        let msg = cmd.to_ipc_message() + "\n";
        writer
            .write_all(msg.as_bytes())
            .map_err(|e| format!("Failed to send command: {}", e))?;
        writer
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// 设置播放属性监听
    fn observe_playback_properties(&mut self) -> Result<(), String> {
        self.send_command(&MpvCommand::observe_property(1, "time-pos"))?;
        self.send_command(&MpvCommand::observe_property(2, "duration"))?;
        self.send_command(&MpvCommand::observe_property(3, "pause"))?;
        self.send_command(&MpvCommand::observe_property(4, "volume"))?;
        self.send_command(&MpvCommand::observe_property(5, "mute"))?;
        self.send_command(&MpvCommand::observe_property(6, "speed"))?;
        self.send_command(&MpvCommand::observe_property(7, "filename"))?;
        Ok(())
    }

    /// 加载并播放文件
    pub fn load_and_play(&mut self, path: &str) -> Result<(), String> {
        self.send_command(&MpvCommand::loadfile(path, "replace"))
    }

    /// 播放
    pub fn play(&mut self) -> Result<(), String> {
        self.send_command(&MpvCommand::play())
    }

    /// 暂停
    pub fn pause(&mut self) -> Result<(), String> {
        self.send_command(&MpvCommand::pause())
    }

    /// 切换播放/暂停
    pub fn toggle_pause(&mut self) -> Result<(), String> {
        self.send_command(&MpvCommand::cycle_pause())
    }

    /// 停止
    pub fn stop(&mut self) -> Result<(), String> {
        self.send_command(&MpvCommand::stop())
    }

    /// 跳转到指定位置
    pub fn seek(&mut self, secs: f64) -> Result<(), String> {
        self.send_command(&MpvCommand::seek_absolute(secs))
    }

    /// 设置音量
    pub fn set_volume(&mut self, volume: f64) -> Result<(), String> {
        self.send_command(&MpvCommand::set_volume(volume))
    }

    /// 截图到文件
    pub fn screenshot_to(&mut self, path: &str) -> Result<(), String> {
        self.send_command(&MpvCommand::screenshot_to_file(path))
    }

    /// 轮询事件（应在主循环中定期调用）
    pub fn poll_events(&mut self) -> Vec<String> {
        let mut events = Vec::new();
        if let Some(ref rx) = self.event_receiver {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.is_running = false;
                        break;
                    }
                }
            }
        }
        events
    }

    /// 关闭 MPV
    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            // 发送 quit 命令
            let _ = self.send_command(&MpvCommand::quit());
            let _ = child.wait();
        }
        self.stdin_writer = None;
        self.event_receiver = None;
        self.is_running = false;
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
