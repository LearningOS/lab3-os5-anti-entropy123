mod pointer;

use alloc::{
    string::ToString,
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{MapPermission, VirtAddr},
    sbi::console_getchar,
    syscall::pointer::{from_user_ptr_to_slice, from_user_ptr_to_str},
    task::{fork_task, pop_cur_task, run_next_task, switch_task, Task, TaskState},
    timer::{self, get_time_ms},
};

use self::pointer::{from_user_cstring, from_user_ptr};
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
    WaitPid,
    GetPid,
    Read,
    SetPriority,
    Exec,
}
impl Syscall {
    fn from(n: usize) -> Result<Syscall, ()> {
        Ok(match n {
            63 => Self::Read,          // 0x3f
            64 => Self::Write,         // 0x40
            93 => Self::Exit,          // 0x5d
            124 => Self::Yield,        // 0x7c
            140 => Self::SetPriority,  // 0x8c
            169 => Self::GetTimeOfDay, // 0xa9
            172 => Self::GetPid,       // 0xac
            215 => Self::Munmap,       // 0xd7
            220 => Self::Fork,         // 0xdc
            221 => Self::Exec,         // 0xdd
            222 => Self::Mmap,         // 0xde
            260 => Self::WaitPid,      // 0x104
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
    fn handle(&self, task: &Weak<Task>, arg1: usize, arg2: usize, arg3: usize) {
        let ret: SyscallResult = match self {
            Syscall::Write => sys_write(task, arg1, arg2, arg3),
            Syscall::Exit => sys_exit(Task::from_weak(&task), arg1 as i32),
            Syscall::GetTimeOfDay => sys_gettimeofday(task, arg1, arg2),
            Syscall::Yield => sys_yield(Task::from_weak(&task)),
            Syscall::TaskInfo => sys_taskinfo(task, arg1),
            Syscall::Mmap => sys_mmap(task, arg1, arg2, arg3),
            Syscall::Munmap => sys_unmmap(task, arg1, arg2),
            Syscall::Fork => sys_fork(task),
            Syscall::WaitPid => sys_waitpid(task, arg1 as isize, arg2),
            Syscall::GetPid => sys_getpid(task),
            Syscall::Read => sys_read(task.upgrade().unwrap(), arg1, arg2, arg3),
            Syscall::SetPriority => sys_set_priority(task, arg1 as isize),
            Syscall::Exec => sys_exec(task, arg1),
            // _ => todo!("unsupported syscall handle function, syscall={:?}", self),
        };
        let ret = ret.unwrap_or(-1);
        let task = Task::from_weak(&task);
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

pub fn syscall_handler(weak_task: &Weak<Task>) {
    let (syscall_num, a0, a1, a2) = {
        let task = Task::from_weak(&weak_task);
        let mut inner = task.inner_exclusive_access();
        let trap_ctx = inner.trap_context();
        let syscall_num = trap_ctx.reg_a(7);
        let ret = (
            syscall_num,
            trap_ctx.reg_a(0),
            trap_ctx.reg_a(1),
            trap_ctx.reg_a(2),
        );
        inner.syscall_times[syscall_num] += 1;
        ret
    };

    let syscall =
        Syscall::from(syscall_num).unwrap_or_else(|_| sys_exit(pop_cur_task().unwrap(), 1));

    {
        let task = Task::from_weak(&weak_task);
        log::info!(
            "{} syscall_handler, num={}, name={:?}",
            task,
            syscall_num,
            syscall
        );
    }
    // log::info!("syscall_times={:?}", ctx.syscall_times);
    syscall.handle(weak_task, a0, a1, a2)
}

fn sys_write(task: &Weak<Task>, fd: usize, buf: usize, len: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
    let user_buf = from_user_ptr_to_str(&task, buf, len);

    log::info!("sys_write args, fd={}, buf=0x{:x}, len={}", fd, buf, len);
    if fd != STDOUT {
        unimplemented!()
    }
    print!("{}", user_buf);
    Ok(len as isize)
}

fn sys_gettimeofday(task: &Weak<Task>, timeval_ptr: usize, _tz: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
    let time = from_user_ptr(&task, timeval_ptr);
    timer::set_time_val(time);
    Ok(0)
}

fn sys_yield(task: Arc<Task>) -> ! {
    {
        task.inner_exclusive_access().set_state(TaskState::Ready)
    }
    switch_task(task)
}

pub fn sys_exit(task: Arc<Task>, exit_code: i32) -> ! {
    {
        let mut inner = task.inner_exclusive_access();
        inner.set_state(TaskState::Exited);
        inner.exit_code = exit_code;
    }
    log::info!(
        "{}, ready to exit, exit_code={}, Arc count={}",
        task,
        exit_code,
        Arc::strong_count(&task)
    );
    drop(task);
    run_next_task()
}

#[derive(Debug)]
pub struct TaskInfo {
    pub state: TaskState,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub exec_time: usize,
}

fn sys_taskinfo(task: &Weak<Task>, user_info: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
    let syscall_times = {
        let inner = task.inner_exclusive_access();
        inner.syscall_times
    };
    let taskinfo = from_user_ptr(&task, user_info);
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

fn sys_mmap(task: &Weak<Task>, start: usize, len: usize, port: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
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
    };
    let mut inner = task.inner_exclusive_access();
    inner
        .addr_space
        .insert_framed_area(start, end, perm)
        .map(|_| 0)
}

fn sys_unmmap(task: &Weak<Task>, start: usize, len: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
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
    let mut inner = task.inner_exclusive_access();
    inner
        .addr_space
        .unmap_area(task.pid.clone(), start, end)
        .map(|_| 0)
}

fn sys_fork(task: &Weak<Task>) -> SyscallResult {
    let task = Task::from_weak(&task);
    let child = fork_task(&task);
    let child_pid = child.pid.0;
    task.inner_exclusive_access().children.push(child);
    Ok(child_pid as isize)
}

fn sys_waitpid(task: &Weak<Task>, target_pid: isize, exit_code: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
    let target_children_pid = {
        let children = { &task.inner_exclusive_access().children };

        let target_children: Vec<&Arc<Task>> = if target_pid == -1 {
            children.iter().collect()
        } else {
            children
                .iter()
                .filter(|t| t.pid.0 == target_pid as usize)
                .collect()
        };
        if target_children.is_empty() {
            return Ok(-1);
        }
        let exited_children: Vec<&Arc<Task>> = target_children
            .iter()
            .map(|t| *t)
            .filter(|t| t.inner_exclusive_access().state == TaskState::Exited)
            .collect();

        if exited_children.is_empty() {
            return Ok(-2);
        }
        (*exited_children.get(0).unwrap()).pid.clone()
    };

    let target_child = {
        let mut inner = task.inner_exclusive_access();
        let (idx, _) = inner
            .children
            .iter()
            .enumerate()
            .find(|(_, t)| t.pid == target_children_pid)
            .expect("should have this pid child");
        inner.children.remove(idx)
    };
    log::info!(
        "{}, have been wait, arc count={}",
        target_child,
        Arc::strong_count(&target_child)
    );

    let exit_code: &mut i32 = from_user_ptr(&task, exit_code);
    *exit_code = {
        let child_inner = target_child.inner_exclusive_access();
        assert!(target_child.pid == target_children_pid);
        assert!(child_inner.state == TaskState::Exited);
        child_inner.exit_code
    };

    Ok(target_children_pid.0 as isize)
}

fn sys_getpid(task: &Weak<Task>) -> SyscallResult {
    let task = Task::from_weak(&task);
    Ok(task.pid.0 as isize)
}

const FD_STDIN: usize = 0;

fn sys_read(task: Arc<Task>, fd: usize, buf: usize, len: usize) -> SyscallResult {
    // let task = Task::from_weak(&task);
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    drop(task);
                    switch_task(pop_cur_task().unwrap());
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let buffer: &mut [u8] = from_user_ptr_to_slice(&task, buf, len);
            buffer[0] = ch;
            Ok(1)
        }
        _ => {
            log::error!("{}, wrong fd? fd={}", task, fd);
            Err(())
        }
    }
}

fn sys_set_priority(_task: &Weak<Task>, priority: isize) -> SyscallResult {
    // let task = Task::from_weak(task);
    Ok(if priority > 1 { priority } else { -1 })
}

fn sys_exec(task: &Weak<Task>, path: usize) -> SyscallResult {
    let task = Task::from_weak(&task);
    let path = from_user_cstring(&task, path);
    log::info!("{}, exec {}", task, path);
    task.exec(&path)?;
    drop(path);
    drop(task);
    switch_task(pop_cur_task().unwrap());
}
