use crate::{
    config::*,
    loader::{get_num_app, setup_task_cx},
    task::{Task, TaskState},
    timer::get_time_ms,
};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref TM: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub struct TaskManager {
    pub next_task: usize,
    task_list: [Option<usize>; MAX_APP_NUM],
}

impl TaskManager {
    fn new() -> Self {
        let mut task_list: [Option<usize>; MAX_APP_NUM] = [None; MAX_APP_NUM];
        for i in 0..get_num_app() {
            let task_ptr = setup_task_cx(i);
            task_list[i] = Some(task_ptr);
        }
        Self {
            task_list,
            next_task: 0,
        }
    }

    pub fn find_next_ready_task(&mut self) -> Option<&mut Task> {
        let current = self.next_task;
        for i in current..(current + MAX_APP_NUM) {
            let app_id = i % 19;
            let task = match self.task_list[app_id].map(|ptr| unsafe { &mut *(ptr as *mut Task) }) {
                None => continue,
                Some(task) => task,
            };
            if task.state() == TaskState::UnInit {
                task.from(i);
                task.start_time_ms = get_time_ms();
            }
            if task.state() == TaskState::Ready {
                self.next_task = app_id + 1;
                task.set_state(TaskState::Running);
                return Some(task);
            }
        }
        None
    }
}
