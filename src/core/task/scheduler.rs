use std::sync::{Arc, Condvar, Mutex};

/// 简易信号量：用于限制最大并发任务数。
/// 使用 `Mutex<usize>` + `Condvar` 实现，适配 `std::thread` 模型。
#[derive(Clone)]
pub struct Semaphore {
    inner: Arc<SemaphoreInner>,
}

struct SemaphoreInner {
    permits: Mutex<usize>,
    /// Condvar 在 permits 变为 > 0 时通知等待线程
    condvar: Condvar,
}

impl Semaphore {
    /// 创建一个具有 `max_permits` 个许可的信号量
    pub fn new(max_permits: usize) -> Self {
        assert!(max_permits > 0, "max_permits must be greater than 0");
        Self {
            inner: Arc::new(SemaphoreInner {
                permits: Mutex::new(max_permits),
                condvar: Condvar::new(),
            }),
        }
    }

    /// 获取一个许可，如果当前没有可用许可则阻塞等待
    pub fn acquire(&self) {
        let mut permits = self.inner.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.inner.condvar.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    /// 尝试获取许可，若不可用则立即返回 false
    pub fn try_acquire(&self) -> bool {
        let mut permits = self.inner.permits.lock().unwrap();
        if *permits > 0 {
            *permits -= 1;
            true
        } else {
            false
        }
    }

    /// 释放一个许可，通知一个等待的线程
    pub fn release(&self) {
        let mut permits = self.inner.permits.lock().unwrap();
        *permits += 1;
        self.inner.condvar.notify_one();
    }

    /// 当前可用许可数
    pub fn available(&self) -> usize {
        *self.inner.permits.lock().unwrap()
    }
}

/// 并发调度器：包装 Semaphore，提供更高层的 submit 接口
pub struct Scheduler {
    semaphore: Semaphore,
    max_concurrent: usize,
}

impl Scheduler {
    /// 创建调度器，`max_concurrent` 控制最大同时执行的任务数
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
            max_concurrent,
        }
    }

    /// 最大并发数
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// 可用许可数
    pub fn available_permits(&self) -> usize {
        self.semaphore.available()
    }

    /// 获取 Semaphore 引用，供 Manager 在 spawn 线程前使用
    pub fn semaphore(&self) -> &Semaphore {
        &self.semaphore
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        // 默认并发数：CPU 核心数，至少 1
        let n = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(n)
    }
}
