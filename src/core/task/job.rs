use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// 全局自增 Job ID
static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

/// 任务类型
#[derive(Debug, Clone, PartialEq)]
pub enum JobType {
    Convert,
    Clip,
    Slice,
    Render,
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Convert => write!(f, "Convert"),
            Self::Clip => write!(f, "Clip"),
            Self::Slice => write!(f, "Slice"),
            Self::Render => write!(f, "Render"),
        }
    }
}

/// 任务状态
#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    /// 等待执行
    Pending,
    /// 正在运行，携带启动时间
    Running { started_at: Instant },
    /// 已完成
    Completed,
    /// 失败，携带错误信息
    Failed { error: String },
    /// 被用户取消
    Cancelled,
}

impl JobStatus {
    /// 是否为终态（不会再变化的状态）
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed { .. } | Self::Cancelled)
    }

    /// 返回状态的展示文本
    pub fn display(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running { .. } => "Running",
            Self::Completed => "Completed",
            Self::Failed { .. } => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}

/// 任务完成后的结果
#[derive(Debug, Clone)]
pub struct JobResult {
    /// 输出文件路径
    pub output_path: Option<PathBuf>,
    /// 任务耗时（秒）
    pub elapsed_secs: f64,
    /// 错误信息（仅失败时）
    pub error_message: Option<String>,
}

/// FFmpeg 转码任务参数
#[derive(Debug, Clone)]
pub struct TranscodeParams {
    pub encoder: String,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_subtitle: bool,
    pub container: String,         // 输出容器格式，如 "mp4"
    pub pix_fmt: String,           // 像素格式
    pub bitrate: String,           // 比特率
    pub quality: String,           // CRF / 质量值
    pub preset: String,            // 编码预设
    pub gop: String,               // GOP 间隔
    pub input_path: PathBuf,
    pub output_dir: Option<PathBuf>,
}

/// 任务定义
#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
    pub job_type: JobType,
    pub params: TranscodeParams,
    pub status: JobStatus,
    /// 进度百分比 0.0 ~ 100.0
    pub progress: f32,
}

impl Job {
    /// 创建新任务，初始状态为 Pending
    pub fn new(job_type: JobType, params: TranscodeParams) -> Self {
        let id = NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            job_type,
            params,
            status: JobStatus::Pending,
            progress: 0.0,
        }
    }

    /// 输入文件名（用于 UI 展示）
    pub fn input_name(&self) -> String {
        self.params
            .input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// 输出文件名（推断）
    pub fn output_name(&self) -> String {
        let stem = self
            .params
            .input_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("output");
        format!("{}.{}", stem, self.params.container)
    }

    // ── 状态转换 ──

    /// Pending → Running
    pub fn mark_running(&mut self) -> Result<(), String> {
        if self.status != JobStatus::Pending {
            return Err(format!(
                "Job {}: can only transition to Running from Pending, current: {:?}",
                self.id, self.status
            ));
        }
        self.status = JobStatus::Running {
            started_at: Instant::now(),
        };
        self.progress = 0.0;
        Ok(())
    }

    /// Running → Completed
    pub fn mark_completed(&mut self) -> Result<(), String> {
        if !matches!(self.status, JobStatus::Running { .. }) {
            return Err(format!(
                "Job {}: can only transition to Completed from Running, current: {:?}",
                self.id, self.status
            ));
        }
        self.status = JobStatus::Completed;
        self.progress = 100.0;
        Ok(())
    }

    /// Running → Failed
    pub fn mark_failed(&mut self, error: String) -> Result<(), String> {
        if !matches!(self.status, JobStatus::Running { .. }) {
            return Err(format!(
                "Job {}: can only transition to Failed from Running, current: {:?}",
                self.id, self.status
            ));
        }
        self.status = JobStatus::Failed { error };
        Ok(())
    }

    /// Pending/Running → Cancelled
    pub fn mark_cancelled(&mut self) -> Result<(), String> {
        if self.status.is_terminal() {
            return Err(format!(
                "Job {}: cannot cancel a terminal job, current: {:?}",
                self.id, self.status
            ));
        }
        self.status = JobStatus::Cancelled;
        Ok(())
    }

    /// 更新进度（从 ffmpeg stderr 解析）
    pub fn update_progress(&mut self, percent: f32) {
        self.progress = percent.clamp(0.0, 100.0);
    }

    /// 获取已运行时长（仅 Running 状态有效）
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        match &self.status {
            JobStatus::Running { started_at } => Some(started_at.elapsed()),
            _ => None,
        }
    }
}
