use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use crate::app::state::FfmpegApp;
use crate::core::processor::ffmpeg::{build_ffmpeg_command, validate_transcode_params};

/// 启动 ffmpeg 进程和读取线程
pub(crate) fn start_ffmpeg(app_state: &mut FfmpegApp) {
    // 清空之前的状态
    app_state.output_lines.clear();
    app_state.error_message = None;
    app_state.is_running = true;

    if let Err(e) = validate_transcode_params(
        &app_state.selected_encoder,
        app_state._is_video,
        app_state._is_audio,
        app_state._is_subtitle,
        &app_state.selected_format,
        &app_state.selected_pixel_format,
        &app_state.bitrate,
        &app_state.constant_rate_factor,
        &app_state.gop,
        app_state.file_path1.as_deref(),
        app_state.folder_path1.as_deref(),
    ) {
        app_state.error_message = format!("Invalid parameter: {}", e).into();
        eprintln!("Invalid parameter: {}", e);
        // 根据错误类型决定是否继续或返回
        return;
    }

    let build_result = build_ffmpeg_command(
        &app_state.selected_encoder,          // encoder: &str
        app_state._is_video,          // bool
        app_state._is_audio,          // bool
        app_state._is_subtitle,       // bool
        &app_state.selected_format,        // container: &str
        &app_state.selected_pixel_format,          // pix_fmt: &str
        &app_state.bitrate,          // bitrate: &str
        &app_state.constant_rate_factor,          // quality: &str
        &app_state.coding_default,           // preset: &str
        &app_state.gop,              // gop: &str
        app_state.file_path1.as_deref(),
        app_state.folder_path1.as_deref(),
    );

    // 创建命令
    let mut cmd: Command = match build_result {
        Ok(c) => c,
        Err(e) => {
            app_state.error_message = Some(format!("参数错误: {}", e));
            app_state.is_running = false;
            return;
        }
    };

    // 隐藏 Windows 控制台窗口
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    // 捕获 stdout 和 stderr
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // 启动子进程
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            app_state.error_message = Some(format!("Failed run ffmpeg: {}", e));
            app_state.is_running = false;
            return;
        }
    };

    // 获取输出句柄
    let stdout = child.stdout.take().expect("Unacquired stdout");
    let stderr = child.stderr.take().expect("Unacquired stderr");

    // 创建通道用于将输出发送到 GUI 线程
    let (tx, rx) = mpsc::channel();
    app_state.receiver = Some(rx);

    // 启动读取线程（分别读取 stdout 和 stderr）
    let tx_clone = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx_clone.send(format!("[stdout] {}", l)).is_err() {
                        break; // 接收端已关闭
                    }
                }
                Err(_) => break,
            }
        }
    });

    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(format!("[stderr] {}", l)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    app_state.child = Some(child);
    app_state.output_lines
        .push(">>> Start run ffmpeg ...".to_string());
}

/// 停止 ffmpeg 进程
pub(crate) fn stop_ffmpeg(app_state: &mut FfmpegApp) {
    if let Some(mut child) = app_state.child.take() {
        let _ = child.kill(); // 强制终止
        let _ = child.wait();
        app_state.output_lines
            .push(">>> User manual termination ffmpeg".to_string());
    }
    app_state.is_running = false;
    app_state.receiver = None;
}