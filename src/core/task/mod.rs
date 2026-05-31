pub mod manager;
pub mod job;
pub mod scheduler;

// 重新导出常用类型，方便外部使用
pub use job::{Job, JobStatus, JobType, TranscodeParams};
pub use manager::TaskManager;
pub use scheduler::Scheduler;