use core::{cell::RefMut, fmt::Display};

use alloc::{
    borrow::ToOwned,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    config::*,
    loader::get_app_elf,
    mm::{MemorySet, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    sync::UPSafeCell,
    timer::get_time_ms,
    trap::TrapContext,
};

use super::{
    add_task, alloc_pid,
    kernel_stack::{alloc_kernel_stack, KernelStack},
    PidHandle,
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TaskState {
    UnInit,
    Ready,
    Running,
    Exited,
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState::UnInit
    }
}

#[repr(C)]
pub struct TaskInner {
    pub trap_ctx_ppn: PhysPageNum,
    pub state: TaskState,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub addr_space: MemorySet,
    pub children: Vec<Arc<Task>>,
    pub exit_code: i32,
}

impl Default for TaskInner {
    fn default() -> Self {
        Self {
            trap_ctx_ppn: Default::default(),
            state: Default::default(),
            syscall_times: [0; MAX_SYSCALL_NUM],
            addr_space: MemorySet::default(),
            children: Vec::new(),
            exit_code: 0,
        }
    }
}

impl TaskInner {
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state
    }

    pub fn translate(&self, va: usize) -> Option<usize> {
        let va = VirtAddr::from(va);
        self.addr_space
            .translate(VirtPageNum::from(va.floor()))
            .map(|entry| PhysAddr::from(entry.ppn()).0 + va.page_offset())
    }

    pub fn trap_context(&self) -> &mut TrapContext {
        self.trap_ctx_ppn.get_mut()
    }
}

#[repr(C, align(4096))]
pub struct Task {
    pub pid: PidHandle,
    pub name: String,
    pub start_time_ms: usize,
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<TaskInner>,
}

impl Task {
    pub fn new(name: &str) -> Arc<Task> {
        let new_pid = alloc_pid();
        let mut task = Task {
            pid: new_pid.clone(),
            name: name.to_owned(),
            start_time_ms: get_time_ms(),
            kernel_stack: alloc_kernel_stack(new_pid),
            inner: unsafe { UPSafeCell::new(TaskInner::default()) },
        };
        task.init(name);
        Arc::new(task)
    }

    fn init(&mut self, name: &str) {
        let kernel_stack_top = self.kernel_stack.position().1;
        let elf_data = get_app_elf(name);
        let (ms, user_stack, entrypoint) = MemorySet::from_elf(elf_data);

        log::debug!(
            "init Task from app_id, &elf_data=0x{:x}, elf_data.len={}, &kernel_stack_top=0x{:x}",
            elf_data.as_ptr() as usize,
            elf_data.len(),
            usize::from(kernel_stack_top)
        );
        let trap_ctx_ppn = ms
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let mut inner = self.inner.exclusive_access();
        inner.trap_ctx_ppn = trap_ctx_ppn;
        inner.addr_space = ms;
        inner.state = TaskState::Ready;
        inner
            .trap_context()
            .init(user_stack, entrypoint, kernel_stack_top)
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskInner> {
        self.inner.exclusive_access()
    }

    pub fn from_weak(weak: &Weak<Self>) -> Arc<Self> {
        weak.upgrade().expect("unexpectly free task control block")
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("task_{}, task_name={}", self.pid, self.name))
    }
}

pub fn fork_task(parent: &Arc<Task>) -> Arc<Task> {
    let new_pid = alloc_pid();
    let p_inner = parent.inner_exclusive_access();
    // init child task
    let child_task = Arc::from(Task {
        pid: new_pid.clone(),
        name: parent.name.clone(),
        start_time_ms: get_time_ms(),
        kernel_stack: alloc_kernel_stack(new_pid.clone()),
        inner: unsafe { UPSafeCell::new(TaskInner::default()) },
    });

    let mut child_inner = child_task.inner_exclusive_access();
    // init basic
    {
        child_inner.syscall_times = [0; MAX_SYSCALL_NUM];
        child_inner.state = TaskState::Ready;
    }
    // init new memory_set
    let trap_ctx_ppn = {
        let ms = MemorySet::from_existed_user(&p_inner.addr_space);
        let trap_ctx_ppn = ms
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        child_inner.addr_space = ms;
        child_inner.trap_ctx_ppn = trap_ctx_ppn;
        trap_ctx_ppn
    };
    // init new trapctx
    {
        let kernel_stack_top = child_task.kernel_stack.position().1;
        trap_ctx_ppn
            .get_bytes_array()
            .copy_from_slice(p_inner.trap_ctx_ppn.get_bytes_array());

        let child_trapctx = child_inner.trap_context();
        child_trapctx.set_reg_a(10, 0); // fork return 0 to child.
        child_trapctx.kernel_sp = kernel_stack_top;
    }
    drop(child_inner);
    add_task(Arc::clone(&child_task));
    child_task
}
