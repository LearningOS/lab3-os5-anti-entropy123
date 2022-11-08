mod manager;
mod task;

use crate::{task::manager::TM, trap::restore};
pub use task::{Task, TaskState};

pub fn run_next_task() -> ! {
    let mut task_manager = TM.lock();
    let task = task_manager
        .find_next_ready_task()
        .expect("all task complete!");

    let cur_task_id = task.id;
    let ctx_ptr = task.get_ptr();

    drop(task_manager);
    log::info!("will run next task, task_idx={}", cur_task_id);
    log::debug!("app_{} ctx_addr=0x{:x}", cur_task_id, ctx_ptr);
    restore(ctx_ptr)
}

pub fn add_initproc() {}
