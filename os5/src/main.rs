#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use sync::UPSafeCell;
use task::add_task;

use crate::task::Task;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate log;

extern crate alloc;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
mod logging;
mod mm;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

core::arch::global_asm!(include_str!("entry.asm"));
core::arch::global_asm!(include_str!("link_app.S"));

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    println!("[kernel] Hello, world!");
    mm::init();
    info!("after mm init!");
    mm::remap_test();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    loader::list_apps();
    // task::add_initproc();
    // info!("after initproc!");
    run_usertest();
    task::run_next_task()
}

lazy_static! {
    pub static ref BATCH_PROCESSING_TASK: UPSafeCell<Vec<Arc<Task>>> =
        unsafe { UPSafeCell::new(Vec::new()) };
}

pub fn run_target_task(names: &[&str]) {
    /* 由于这种产生进程的方式会让它们没有父进程
     * 在 exit 后, 换栈之前, 提前释放进程控制块,
     * 进而释放内核栈, 导致缺页错误. 所以要在这里
     * 留一个引用计数
     */
    let mut batch_processing_task = BATCH_PROCESSING_TASK.exclusive_access();
    for name in names.iter() {
        batch_processing_task.push(Task::new(*name));
    }

    for task in batch_processing_task.iter() {
        add_task(Arc::clone(task))
    }
}

pub fn run_usertest() {
    run_target_task(&["ch5_usertest"]);
    // batch_processing_task.push(Task::new("ch2b_bad_address"));
    // batch_processing_task.push(Task::new("ch2b_hello_world"));
    // batch_processing_task.push(Task::new("ch2b_power_7"));
    // batch_processing_task.push(Task::new("ch3b_sleep1"));
    // batch_processing_task.push(Task::new("ch3_taskinfo"));
    // batch_processing_task.push(Task::new("ch4_mmap3"));
    // batch_processing_task.push(Task::new("ch4_unmap2"));
    // batch_processing_task.push(Task::new("ch5b_exit"));
    // batch_processing_task.push(Task::new("ch5b_forktest_simple"));
    // batch_processing_task.push(Task::new("ch5b_forktest"));
    // batch_processing_task.push(Task::new("ch5_getpid"));
    // batch_processing_task.push(Task::new("ch5b_forktree"));
    // batch_processing_task.push(Task::new("ch5b_forktest2"));
    // batch_processing_task.push(Task::new("ch5_setprio"));
    // batch_processing_task.push(Task::new("ch5_spawn1"));
    // batch_processing_task.push(Task::new("ch5_stride"));
}
