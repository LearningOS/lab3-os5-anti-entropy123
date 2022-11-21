use alloc::{format, string::String, sync::Arc};

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

pub fn from_user_ptr_to_slice<T>(task: &Arc<Task>, buf: usize, len: usize) -> &'static mut [T]
where
    T: Sized,
{
    let buf = task
        .inner_exclusive_access()
        .translate(buf)
        .expect(&format!(
            "task_{}, task_name={}, receive bad user buffer addr? user_buf_addr=0x{:x}",
            task.pid, task.name, buf
        ));

    unsafe { core::slice::from_raw_parts_mut(buf as *mut T, len) }
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

pub fn from_user_cstring(task: &Arc<Task>, user_addr: usize) -> String {
    // let mut user_addr = user_addr;
    let mut phy_addr = task
        .inner_exclusive_access()
        .translate(user_addr)
        .expect(&format!(
            "task_{}, task_name={}, receive bad user addr? user_addr=0x{:x}",
            task.pid, task.name, user_addr
        ));

    let mut s = String::new();
    loop {
        unsafe {
            let val = *(phy_addr as *const u8);
            if val == 0 {
                break;
            }
            phy_addr += 1;
            s.push(val as char);
        }
    }
    s
}
