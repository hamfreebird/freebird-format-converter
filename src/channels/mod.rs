use crate::app::state::FfmpegApp;
use crate::channels::messages::UiMessages;
use crate::core;
use crate::core::task::{TaskManager, TranscodeParams};

pub mod messages;

pub fn process_message(state: &mut FfmpegApp, msg: UiMessages) {
    match msg {
        UiMessages::PickFile(_id) => {
            core::utils::scanner::pick_file(
                &mut state.file_picker_tx,
                &mut state.file_picker_rx,
                &mut state.active_picker,
                1
            );
        },
        UiMessages::PickFolder(_id) => {
            core::utils::scanner::pick_folder(
                &mut state.folder_picker_tx,
                &mut state.folder_picker_rx,
                &mut state.active_folder_picker,
                1
            );
        },
        UiMessages::StartFFMPEG => {
            // 通过任务系统提交转码任务
            submit_convert_job(state);
        },
        UiMessages::StopFFMPEG => {
            crate::services::ffmpeg_service::stop_ffmpeg(state);
        },

        // ── UI 控制 ──
        UiMessages::ToggleSettings => {
            state.show_settings = !state.show_settings;
        }
        UiMessages::ToggleTaskPanel => {
            state.show_task_panel = !state.show_task_panel;
        }
        UiMessages::ToggleAbout => {
            state.show_about = !state.show_about;
        }
        UiMessages::SwitchWindow(target) => {
            state.window_state = target;
        }
        UiMessages::CheckFFmpegVersion => {
            // 同步调用 ffmpeg -version 获取版本信息
            match std::process::Command::new("ffmpeg")
                .arg("-version")
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    state.ffmpeg_version_text = if stdout.is_empty() {
                        stderr
                    } else {
                        stdout
                    };
                }
                Err(e) => {
                    state.ffmpeg_version_text = format!(
                        "Failed to run ffmpeg -version: {}\n\
                         Please make sure ffmpeg is installed and in your PATH.",
                        e
                    );
                }
            }
            state.show_ffmpeg_version = true;
        }

        // ── 元数据 ──
        UiMessages::MetadataLoaded(result) => {
            match result {
                Ok(meta) => {
                    state.output_lines.push(format!(
                        ">>> Metadata: {}", meta.summary()
                    ));
                    state.metadata = Some(meta);
                }
                Err(e) => {
                    state.output_lines.push(format!(">>> Metadata error: {}", e));
                    state.metadata = None;
                }
            }
        }

        // ── 任务系统消息处理 ──
        UiMessages::JobProgress { job_id, percent } => {
            if let Some(ref tm) = state.task_manager {
                if let Some(job) = tm.all_jobs().iter().find(|j| {
                    j.lock().unwrap().id == job_id
                }) {
                    let mut j = job.lock().unwrap();
                    j.update_progress(percent);
                }
            }
        }
        UiMessages::JobComplete { job_id } => {
            if let Some(ref tm) = state.task_manager {
                if let Some(job) = tm.all_jobs().iter().find(|j| {
                    j.lock().unwrap().id == job_id
                }) {
                    let mut j = job.lock().unwrap();
                    let _ = j.mark_completed();
                }
            }
        }
        UiMessages::JobError { job_id, error } => {
            if let Some(ref tm) = state.task_manager {
                if let Some(job) = tm.all_jobs().iter().find(|j| {
                    j.lock().unwrap().id == job_id
                }) {
                    let mut j = job.lock().unwrap();
                    let _ = j.mark_failed(error);
                }
            }
        }
        UiMessages::TryDispatchNext => {
            if let Some(ref mut tm) = state.task_manager {
                tm.on_try_dispatch_next();
            }
        }

        _ => {}
    }
}

/// 从当前应用状态构建 TranscodeParams 并提交到任务管理器
fn submit_convert_job(state: &mut FfmpegApp) {
    // 延迟初始化 TaskManager
    if state.task_manager.is_none() {
        let sender = state.inbox.sender();
        state.task_manager = Some(TaskManager::new(sender));
    }

    let input_path = match &state.file_path1 {
        Some(p) => p.clone(),
        None => {
            state.error_message = Some("Please select an input file".to_string());
            return;
        }
    };

    let output_dir = state.folder_path1.clone();

    let params = TranscodeParams {
        encoder: state.selected_encoder.clone(),
        is_video: state._is_video,
        is_audio: state._is_audio,
        is_subtitle: state._is_subtitle,
        container: state.selected_format.clone(),
        pix_fmt: state.selected_pixel_format.clone(),
        bitrate: state.bitrate.clone(),
        quality: state.constant_rate_factor.clone(),
        preset: state.coding_default.clone(),
        gop: state.gop.clone(),
        input_path,
        output_dir,
    };

    if let Some(ref mut tm) = state.task_manager {
        let job_id = tm.enqueue_convert(params);
        state.output_lines.push(format!(
            ">>> Job #{} enqueued: {}",
            job_id,
            state.file_path1.as_ref().unwrap().display()
        ));
    }
}