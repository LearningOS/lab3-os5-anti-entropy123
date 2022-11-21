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
        if self.task_list.is_empty() {
            return None;
        }
        let mut smallest_pass_idx = 0;
        let mut smallest_pass = self.task_list[0].inner_exclusive_access().pass;
        for i in 1..self.task_list.len() {
            let pass = self.task_list[i].inner_exclusive_access().pass;
            if pass < smallest_pass {
                smallest_pass_idx = i;
                smallest_pass = pass;
            }
        }

        Some(
            self.task_list
                .remove(smallest_pass_idx)
                .expect("wrong smallest_pass_idx?"),
        )
    }

    pub fn add_task(&mut self, task: Arc<Task>) {
        self.task_list.push_back(task);
    }
}
