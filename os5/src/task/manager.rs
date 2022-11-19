use crate::task::{Task, TaskState};
use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref TM: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub struct TaskManager {
    pub next_task: usize,
    task_list: VecDeque<Arc<Task>>,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            task_list: VecDeque::new(),
            next_task: 0,
        }
    }

    pub fn find_next_ready_task(&mut self) -> Option<Arc<Task>> {
        let task = match self.task_list.pop_front() {
            None => return None,
            Some(task) => task,
        };
        assert!(task.inner_exclusive_access().state == TaskState::Ready);
        Some(task)
    }

    pub fn add_task(&mut self, task: Arc<Task>) {
        self.task_list.push_back(task);
    }
}
