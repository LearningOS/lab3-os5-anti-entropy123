mod manager;
mod pid;
mod processor;
mod task;

use alloc::sync::Arc;

use crate::{
    task::{manager::TM, processor::processor_inner},
    trap::restore,
};
pub use {
    pid::{alloc_pid, PidHandle},
    task::{fork_task, Task, TaskInner, TaskState},
};

// 将初始进程加入任务管理器.
pub fn add_initproc() {
    TM.lock().add_task(Task::new("ch5b_initproc"))
}

pub fn add_task(task: Arc<Task>) {
    TM.lock().add_task(task)
}

pub fn fetch_ready_task() -> Arc<Task> {
    let mut task_manager = TM.lock();
    let task = task_manager
        .find_next_ready_task()
        .expect("all task complete!");
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
    if task.pid.0 == 2 {
        log::warn!("locate this task!")
    }
    run_task(task)
}

pub fn switch_task(previous_task: Arc<Task>) -> ! {
    add_task(previous_task);
    run_next_task();
}

pub fn pop_task() -> Option<Arc<Task>> {
    processor_inner().pop_task()
}
