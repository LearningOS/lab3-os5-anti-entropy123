mod kernel_stack;
mod manager;
mod pid;
mod processor;
mod task;

use alloc::sync::{Arc, Weak};

use crate::{
    task::{manager::TM, processor::processor_inner},
    trap::restore,
    BATCH_PROCESSING_TASK,
};
pub use {
    pid::{alloc_pid, PidHandle},
    task::{fork_task, Task, TaskInner, TaskState},
};

// 将初始进程加入任务管理器.
#[allow(dead_code)]
pub fn add_initproc() {
    TM.lock().add_task(Task::new("ch5b_initproc"))
}

pub fn add_task(task: Arc<Task>) {
    TM.lock().add_task(task)
}

pub fn fetch_ready_task() -> Arc<Task> {
    let mut task_manager = TM.lock();
    let task = match task_manager.find_next_ready_task() {
        None => {
            let mut batch_tasks = BATCH_PROCESSING_TASK.exclusive_access();
            while batch_tasks.len() > 0 {
                batch_tasks.pop();
            }
            panic!("all task complete!");
        }
        Some(task) => task,
    };

    task
}

pub fn run_task(task: Arc<Task>) -> ! {
    processor_inner().cur_task = Some(Arc::clone(&task));
    restore(task)
}

pub fn run_next_task() -> ! {
    let task = fetch_ready_task();

    log::info!(
        "will run next task, task_pid={}, task_name={}",
        &task.pid,
        &task.name
    );
    run_task(task)
}

pub fn switch_task(previous_task: Arc<Task>) -> ! {
    add_task(previous_task);
    run_next_task();
}

pub fn pop_cur_task() -> Option<Arc<Task>> {
    processor_inner().pop_task()
}

pub fn weak_cur_task() -> Option<Weak<Task>> {
    processor_inner().weak_task()
}
