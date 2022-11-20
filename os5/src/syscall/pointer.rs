use alloc::{format, sync::Arc};

use crate::task::Task;

pub fn from_user_ptr_to_str(task: &Arc<Task>, buf: usize, len: usize) -> &'static str {
    let buf = task
        .inner_exclusive_access()
        .translate(buf)
        .expect(&format!(
            "task_{}, task_name={}, receive bad user buffer addr? user_buf_addr=0x{:x}",
            task.pid, task.name, buf
        ));

    let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, len) };
    core::str::from_utf8(slice).unwrap()
}

pub fn from_user_ptr<T>(task: &Arc<Task>, user_addr: usize) -> &'static mut T {
    let phy_addr = task
        .inner_exclusive_access()
        .translate(user_addr)
        .expect(&format!(
            "task_{}, task_name={}, receive bad user addr? user_addr=0x{:x}",
            task.pid, task.name, user_addr
        ));

    unsafe { &mut *(phy_addr as *mut T) }
}
