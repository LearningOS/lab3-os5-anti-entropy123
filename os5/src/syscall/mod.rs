use alloc::{format, string::ToString, sync::Arc};

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
    Fork,
}
impl Syscall {
    fn from(n: usize) -> Result<Syscall, ()> {
        Ok(match n {
            64 => Self::Write,         // 0x40
            93 => Self::Exit,          // 0x5d
            124 => Self::Yield,        // 0x7c
            169 => Self::GetTimeOfDay, // 0xa9
            215 => Self::Munmap,       // 0xd7
            220 => Self::Fork,         // 0xdc
            222 => Self::Mmap,         // 0xde
            410 => Self::TaskInfo,     // 0x19a
            _ => {
                log::warn!("unsupported syscall: {}", n.to_string());
                return Err(());
            }
        })
    }
}

type SyscallResult = Result<isize, ()>;

impl Syscall {
    fn handle(&self, task: Arc<Task>, arg1: usize, arg2: usize, arg3: usize) {
        let ret: SyscallResult = match self {
            Syscall::Write => sys_write(Arc::clone(&task), arg1, arg2, arg3),
            Syscall::Exit => sys_exit(task),
            Syscall::GetTimeOfDay => sys_gettimeofday(Arc::clone(&task), arg1, arg2),
            Syscall::Yield => sys_yield(task),
            Syscall::TaskInfo => sys_taskinfo(Arc::clone(&task), arg1),
            Syscall::Mmap => sys_mmap(Arc::clone(&task), arg1, arg2, arg3),
            Syscall::Munmap => sys_unmmap(Arc::clone(&task), arg1, arg2),
            Syscall::Fork => sys_fork(Arc::clone(&task)),
            _ => todo!("unsupported syscall handle function, syscall={:?}", self),
        };
        let ret = ret.unwrap_or(-1);
        let a0 = {
            let inner = task.inner_exclusive_access();
            let trap_ctx = inner.trap_context();
            trap_ctx.set_reg_a(0, ret as usize);
            trap_ctx.reg_a(0)
        };
        log::info!(
            "task_{} syscall ret={:x}, task.trap_ctx.x[10]={:x}",
            task.pid,
            ret,
            a0
        );
    }
}

pub fn syscall_handler(ctx: Arc<Task>) {
    let (syscall_num, a0, a1, a2) = {
        let inner = ctx.inner_exclusive_access();
        let trap_ctx = inner.trap_context();
        (
            trap_ctx.reg_a(7),
            trap_ctx.reg_a(0),
            trap_ctx.reg_a(1),
            trap_ctx.reg_a(2),
        )
    };
    {
        ctx.inner_exclusive_access().syscall_times[syscall_num] += 1;
    }
    let syscall = Syscall::from(syscall_num).unwrap_or_else(|_| sys_exit(Arc::clone(&ctx)));

    log::info!(
        "task_{}({}) syscall_handler, num={}, name={:?}",
        ctx.pid,
        ctx.name,
        syscall_num,
        syscall
    );
    // log::info!("syscall_times={:?}", ctx.syscall_times);
    syscall.handle(ctx, a0, a1, a2)
}

fn sys_write(task: Arc<Task>, fd: usize, buf: usize, len: usize) -> SyscallResult {
    let buf = task
        .inner_exclusive_access()
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
    Ok(len as isize)
}

fn sys_gettimeofday(task: Arc<Task>, timeval_ptr: usize, _tz: usize) -> SyscallResult {
    let inner = task.inner_exclusive_access();
    let timeval_ptr = inner.translate(timeval_ptr).expect(&format!(
        "sys_gettimeofday, receive bad timeval_ptr addr? buf=0x{:x}",
        timeval_ptr
    ));

    let time = unsafe { &mut *(timeval_ptr as *mut TimeVal) };
    timer::set_time_val(time);
    Ok(0)
}

fn sys_yield(task: Arc<Task>) -> ! {
    {
        task.inner_exclusive_access().set_state(TaskState::Ready)
    }
    run_next_task();
}

pub fn sys_exit(task: Arc<Task>) -> ! {
    {
        task.inner_exclusive_access().set_state(TaskState::Exited)
    }
    run_next_task()
}

#[derive(Debug)]
pub struct TaskInfo {
    pub state: TaskState,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub exec_time: usize,
}

fn sys_taskinfo(task: Arc<Task>, user_info: usize) -> SyscallResult {
    let (user_info, syscall_times) = {
        let inner = task.inner_exclusive_access();
        (
            inner.translate(user_info).expect(&format!(
                "task_{}({}) sys_taskinfo, receive bad user_info addr? buf=0x{:x}",
                task.pid, task.name, user_info
            )),
            inner.syscall_times,
        )
    };
    let taskinfo = unsafe { &mut *(user_info as *mut TaskInfo) };
    *taskinfo = TaskInfo {
        state: TaskState::Running,
        syscall_times,
        exec_time: get_time_ms() - task.start_time_ms,
    };
    log::debug!(
        "task_{}({}) sys_taskinfo, copyout user_info={:?}",
        task.pid,
        task.name,
        taskinfo
    );
    Ok(0)
}

fn sys_mmap(task: Arc<Task>, start: usize, len: usize, port: usize) -> SyscallResult {
    log::info!(
        "task_{}({}) sys_mmap, receive args start=0x{:x}, end=0x{:x}, len=0x{:x}, port=0x{:x}",
        task.pid,
        task.name,
        start,
        start + len,
        len,
        port
    );
    if port & !0x7 != 0 {
        log::info!(
            "task_{}({}) sys_mmap failed, receive bad port? port=0x{:x}",
            task.pid,
            task.name,
            port
        );
        return Err(());
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
                    "task_{}({}) sys_mmap failed, receive meaningless port? port=0x{:x}",
                    task.pid,
                    task.name,
                    port
                );
                return Err(());
            }
        };

    let end = VirtAddr::from(start + len);
    let start = VirtAddr::from(start);
    if start.page_offset() != 0 {
        return Err(());
    }
    task.inner_exclusive_access()
        .addr_space
        .insert_framed_area(start, end, perm)
        .map(|_| 0)
}

fn sys_unmmap(task: Arc<Task>, start: usize, len: usize) -> SyscallResult {
    log::info!(
        "task_{}({}) sys_unmmap, receive args start=0x{:x}, len=0x{:x}",
        task.pid,
        task.name,
        start,
        len
    );
    let end = VirtAddr::from(start + len);
    let start = VirtAddr::from(start);
    if start.page_offset() != 0 {
        return Err(());
    }
    task.inner_exclusive_access()
        .addr_space
        .unmap_area(task.pid.clone(), start, end)
        .map(|_| 0)
}

fn sys_fork(_task: Arc<Task>) -> SyscallResult {
    todo!()
}
