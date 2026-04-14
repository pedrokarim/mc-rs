//! Async task pool.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct AsyncTaskInfo {
    pub id: u64,
    pub name: String,
    pub state: TaskState,
    pub progress: f32,
}

impl AsyncTaskInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            state: TaskState::Pending,
            progress: 0.0,
        }
    }
}

/// Max parallel tasks (4 for I/O, 2 for CPU work).
pub const MAX_IO_TASKS: usize = 4;
pub const MAX_CPU_TASKS: usize = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_unique() {
        let a = AsyncTaskInfo::new("a");
        let b = AsyncTaskInfo::new("b");
        assert_ne!(a.id, b.id);
    }
}
