use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use egui_inbox::UiInboxSender;

use crate::channels::messages::UiMessages;
use crate::core::task::job::{Job, JobType, TranscodeParams};
use crate::core::task::scheduler::Scheduler;

/// 共享的 Job 引用，允许跨线程读写 Job 状态
pub type SharedJob = Arc<Mutex<Job>>;

/// 任务队列管理器：负责排队、并发控制和状态跟踪
pub struct TaskManager {
    /// 等待执行的任务队列
    queue: VecDeque<SharedJob>,
    /// 所有已知任务（含活跃和已完成的）
    all_jobs: Vec<SharedJob>,
    /// 并发调度器
    scheduler: Scheduler,
    /// 发送 UI 消息的通道
    ui_sender: UiInboxSender<UiMessages>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new(ui_sender: UiInboxSender<UiMessages>) -> Self {
        Self {
            queue: VecDeque::new(),
            all_jobs: Vec::new(),
            scheduler: Scheduler::default(),
            ui_sender,
        }
    }

    /// 使用自定义并发数创建
    pub fn with_max_concurrent(ui_sender: UiInboxSender<UiMessages>, max_concurrent: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            all_jobs: Vec::new(),
            scheduler: Scheduler::new(max_concurrent),
            ui_sender,
        }
    }

    /// 将一个转码任务入队，返回 job id
    pub fn enqueue_convert(&mut self, params: TranscodeParams) -> u64 {
        let job = Job::new(JobType::Convert, params);
        let id = job.id;
        let shared = Arc::new(Mutex::new(job));
        self.queue.push_back(Arc::clone(&shared));
        self.all_jobs.push(shared);
        self.try_dispatch_next();
        id
    }

    /// 取消指定任务
    /// - Pending: 直接从队列中移除
    /// - Running: 标记为 Cancelled（子进程由外部 kill）
    pub fn cancel(&mut self, job_id: u64) -> Option<String> {
        // 先在队列中查找
        if let Some(pos) = self.queue.iter().position(|j| j.lock().unwrap().id == job_id) {
            let job = self.queue.remove(pos).unwrap();
            let mut j = job.lock().unwrap();
            let _ = j.mark_cancelled();
            return Some(format!("Job {} ({}) cancelled", job_id, j.input_name()));
        }

        // 再在活跃列表中查找
        if let Some(job) = self.all_jobs.iter().find(|j| j.lock().unwrap().id == job_id) {
            let mut j = job.lock().unwrap();
            if !j.status.is_terminal() {
                let _ = j.mark_cancelled();
                return Some(format!("Job {} ({}) cancelled", job_id, j.input_name()));
            }
        }

        None
    }

    /// 清除所有已完成/失败/取消的任务
    pub fn clear_completed(&mut self) {
        self.all_jobs.retain(|j| {
            let j = j.lock().unwrap();
            !j.status.is_terminal()
        });
    }

    /// 获取所有任务（用于 UI 渲染）
    pub fn all_jobs(&self) -> &Vec<SharedJob> {
        &self.all_jobs
    }

    /// 队列中等待的任务数
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// 当前可用并发槽位数
    pub fn available_slots(&self) -> usize {
        self.scheduler.available_permits()
    }

    /// 最大并发数
    pub fn max_concurrent(&self) -> usize {
        self.scheduler.max_concurrent()
    }

    // ── 内部方法 ──

    /// 尝试从队列中取出下一个任务并派发执行
    fn try_dispatch_next(&mut self) {
        while let Some(job) = self.queue.pop_front() {
            // 检查任务是否已被取消
            {
                let j = job.lock().unwrap();
                if j.status.is_terminal() {
                    continue; // 跳过已取消的任务
                }
            }

            // 尝试获取信号量许可
            if self.scheduler.semaphore().try_acquire() {
                self.dispatch_job(Arc::clone(&job));
            } else {
                // 无法获取许可，放回队列前端
                self.queue.push_front(job);
                break;
            }
        }
    }

    /// 派发单个任务到新线程执行
    fn dispatch_job(&self, job: SharedJob) {
        let sem = self.scheduler.semaphore().clone();
        let sender = self.ui_sender.clone();

        // 标记为 Running
        {
            let mut j = job.lock().unwrap();
            let _ = j.mark_running();
        }

        let job_clone = Arc::clone(&job);
        let job_id = job.lock().unwrap().id;

        std::thread::spawn(move || {
            // 发送进度更新
            let _ = sender.send(UiMessages::JobProgress {
                job_id,
                percent: 0.0,
            });

            // --- 在此执行 FFmpeg 转码（由 ffmpeg_service 填充实际逻辑） ---
            // 目前占位：模拟执行
            let result = execute_transcode_job(&job_clone, &sender);

            // 释放信号量许可
            sem.release();

            // 发送完成/失败消息
            match result {
                Ok(()) => {
                    let _ = sender.send(UiMessages::JobComplete { job_id });
                }
                Err(e) => {
                    let _ = sender.send(UiMessages::JobError {
                        job_id,
                        error: e,
                    });
                }
            }

            // 通知 Manager 尝试派发下一个任务
            let _ = sender.send(UiMessages::TryDispatchNext);
        });
    }

    /// 当收到 TryDispatchNext 时由外部调用（在 process_message 中处理）
    pub fn on_try_dispatch_next(&mut self) {
        self.try_dispatch_next();
    }
}

/// 执行单个转码任务（占位实现，后续由 ffmpeg_service 取代）
fn execute_transcode_job(
    job: &SharedJob,
    _sender: &UiInboxSender<UiMessages>,
) -> Result<(), String> {
    let params = {
        let j = job.lock().unwrap();
        j.params.clone()
    };

    // 使用现有的 build_ffmpeg_command 和 validate_transcode_params
    crate::core::processor::ffmpeg::validate_transcode_params(
        &params.encoder,
        params.is_video,
        params.is_audio,
        params.is_subtitle,
        &params.container,
        &params.pix_fmt,
        &params.bitrate,
        &params.quality,
        &params.gop,
        Some(&params.input_path),
        params.output_dir.as_deref(),
    )?;

    let mut cmd = crate::core::processor::ffmpeg::build_ffmpeg_command(
        &params.encoder,
        params.is_video,
        params.is_audio,
        params.is_subtitle,
        &params.container,
        &params.pix_fmt,
        &params.bitrate,
        &params.quality,
        &params.preset,
        &params.gop,
        Some(&params.input_path),
        params.output_dir.as_deref(),
    )
    .map_err(|e| format!("Failed to build ffmpeg command: {}", e))?;

    // 执行 FFmpeg
    let output = cmd.output().map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if output.status.success() {
        let mut j = job.lock().unwrap();
        let _ = j.mark_completed();
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 尝试从 stderr 中提取错误信息（最后几行通常有关键信息）
        let error_lines: Vec<&str> = stderr.lines().rev().take(3).collect();
        let error_msg = error_lines
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        let mut j = job.lock().unwrap();
        let _ = j.mark_failed(error_msg.clone());
        Err(error_msg)
    }
}
