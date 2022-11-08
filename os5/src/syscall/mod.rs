use alloc::{format, string::ToString};

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{MapPermission, VirtAddr},
    task::{run_next_task, Task, TaskState},
    timer::{self, get_time_ms, TimeVal},
};
const STDOUT: usize = 1;

#[derive(Debug)]
enum Syscall {
    Exit,
    Write,
    GetTimeOfDay,
    Yield,
    TaskInfo,
    Mmap,
    Munmap,
}

impl From<usize> for Syscall {
    fn from(n: usize) -> Self {
        match n {
            64 => Self::Write,         // 0x40
            93 => Self::Exit,          // 0x5d
            124 => Self::Yield,        // 0x7c
            169 => Self::GetTimeOfDay, // 0xa9
            215 => Self::Munmap,       // 0xd7
            222 => Self::Mmap,         // 0xde
            410 => Self::TaskInfo,     // 0x19a
            _ => todo!("unsupported syscall: {}", n.to_string()),
        }
    }
}

impl Syscall {
    fn handle(&self, task: &mut Task, arg1: usize, arg2: usize, arg3: usize) {
        let ret = match self {
            Syscall::Write => sys_write(task, arg1, arg2, arg3),
            Syscall::Exit => sys_exit(task),
            Syscall::GetTimeOfDay => sys_gettimeofday(task, arg1, arg2) as usize,
            Syscall::Yield => sys_yield(task),
            Syscall::TaskInfo => sys_taskinfo(&task, arg1),
            Syscall::Mmap => sys_mmap(task, arg1, arg2, arg3) as usize,
            Syscall::Munmap => sys_unmmap(task, arg1, arg2) as usize,
            _ => todo!("unsupported syscall handle function"),
        };
        task.trap_ctx.set_reg_a(0, ret);
        log::info!(
            "task_{} syscall ret={:x}, task.trap_ctx.x[10]={:x}",
            task.id,
            ret,
            task.trap_ctx.reg_a(0)
        );
    }
}

pub fn syscall_handler(ctx: &mut Task) {
    let trap_ctx = &mut ctx.trap_ctx;
    let (syscall_num, a0, a1, a2) = (
        trap_ctx.reg_a(7),
        trap_ctx.reg_a(0),
        trap_ctx.reg_a(1),
        trap_ctx.reg_a(2),
    );
    ctx.syscall_times[syscall_num] += 1;
    let syscall = Syscall::from(syscall_num);
    log::info!(
        "task_{} syscall_handler, num={}, name={:?}",
        ctx.id,
        syscall_num,
        syscall
    );
    // log::info!("syscall_times={:?}", ctx.syscall_times);
    syscall.handle(ctx, a0, a1, a2)
}

fn sys_write(task: &Task, fd: usize, buf: usize, len: usize) -> usize {
    let buf = task
        .translate(buf)
        .expect(&format!("sys_write, receive bad buf addr? buf=0x{:x}", buf));

    let user_buf = {
        let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, len) };
        core::str::from_utf8(slice).unwrap()
    };

    log::info!("sys_write args, fd={}, buf=0x{:x}, len={}", fd, buf, len);
    if fd != STDOUT {
        unimplemented!()
    }
    print!("{}", user_buf);
    len
}

fn sys_gettimeofday(task: &Task, timeval_ptr: usize, _tz: usize) -> isize {
    let timeval_ptr = task.translate(timeval_ptr).expect(&format!(
        "sys_gettimeofday, receive bad timeval_ptr addr? buf=0x{:x}",
        timeval_ptr
    ));

    let time = unsafe { &mut *(timeval_ptr as *mut TimeVal) };
    timer::set_time_val(time);
    0
}

fn sys_yield(task: &mut Task) -> usize {
    task.set_state(TaskState::Ready);
    run_next_task();
}

pub fn sys_exit(task: &mut Task) -> ! {
    task.set_state(TaskState::Exited);
    run_next_task()
}

#[derive(Debug)]
pub struct TaskInfo {
    pub state: TaskState,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub exec_time: usize,
}

fn sys_taskinfo(task: &Task, user_info: usize) -> usize {
    let user_info = task.translate(user_info).expect(&format!(
        "task_{} sys_taskinfo, receive bad user_info addr? buf=0x{:x}",
        task.id, user_info
    ));
    let taskinfo = unsafe { &mut *(user_info as *mut TaskInfo) };
    *taskinfo = TaskInfo {
        state: TaskState::Running,
        syscall_times: task.syscall_times,
        exec_time: get_time_ms() - task.start_time_ms,
    };
    log::debug!(
        "task_{} sys_taskinfo, copyout user_info={:?}",
        task.id,
        taskinfo
    );
    0
}

fn sys_mmap(task: &mut Task, start: usize, len: usize, port: usize) -> isize {
    log::info!(
        "task_{} sys_mmap, receive args start=0x{:x}, end=0x{:x}, len=0x{:x}, port=0x{:x}",
        task.id,
        start,
        start + len,
        len,
        port
    );
    if port & !0x7 != 0 {
        log::info!(
            "task_{} sys_mmap failed, receive bad port? port=0x{:x}",
            task.id,
            port
        );
        return -1;
    }
    let perm = MapPermission::U
        | match port {
            7 => MapPermission::X | MapPermission::W | MapPermission::R,
            4 => MapPermission::X | MapPermission::R,
            3 => MapPermission::W | MapPermission::R,
            2 => MapPermission::W,
            1 => MapPermission::R,
            _ => {
                log::info!(
                    "task_{} sys_mmap failed, receive meaningless port? port=0x{:x}",
                    task.id,
                    port
                );
                return -1;
            }
        };

    let end = VirtAddr::from(start + len);
    let start = VirtAddr::from(start);
    if start.page_offset() != 0 {
        return -1;
    }
    match task.addr_space.insert_framed_area(start, end, perm) {
        Ok(_) => 0,
        _ => -1,
    }
}

fn sys_unmmap(task: &mut Task, start: usize, len: usize) -> isize {
    log::info!(
        "task_{} sys_unmmap, receive args start=0x{:x}, len=0x{:x}",
        task.id,
        start,
        len
    );
    let end = VirtAddr::from(start + len);
    let start = VirtAddr::from(start);
    if start.page_offset() != 0 {
        return -1;
    }
    match task.addr_space.unmap_area(task.id, start, end) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
