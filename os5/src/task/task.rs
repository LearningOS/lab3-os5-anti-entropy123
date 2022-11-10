use core::cell::RefMut;

use crate::{
    config::*,
    loader::{get_app_elf, get_kernel_stack_phyaddr},
    mm::{MemorySet, PhysAddr, VirtAddr, VirtPageNum},
    sync::UPSafeCell,
    timer::get_time_ms,
    trap::TrapContext,
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TaskState {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[repr(C)]
pub struct TaskInner {
    pub trap_ctx: TrapContext,
    pub state: TaskState,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub addr_space: MemorySet,
}

impl TaskInner {
    pub fn state(&self) -> TaskState {
        self.state
    }

    pub fn set_state(&mut self, state: TaskState) {
        self.state = state
    }

    pub fn translate(&self, va: usize) -> Option<usize> {
        let va = VirtAddr::from(va);
        self.addr_space
            .translate(VirtPageNum::from(va.floor()))
            .map(|entry| PhysAddr::from(entry.ppn()).0 + va.page_offset())
    }
}

#[repr(C)]
pub struct Task {
    pub id: usize,
    pub start_time_ms: usize,
    inner: UPSafeCell<TaskInner>,
}

impl Task {
    pub fn from(&mut self, app_id: usize) {
        let (elf_data, kernel_stack_addr) = (get_app_elf(app_id), get_kernel_stack_phyaddr(app_id));
        log::debug!(
            "init Task from app_id, &elf_data=0x{:x}, elf_data.len={}, &kernel_stack=0x{:x}",
            elf_data.as_ptr() as usize,
            elf_data.len(),
            usize::from(kernel_stack_addr)
        );

        let (ms, user_stack, entrypoint) = MemorySet::from_elf(elf_data, kernel_stack_addr);

        self.id = app_id;
        self.start_time_ms = get_time_ms();
        let mut inner = self.inner_exclusive_access();
        inner.trap_ctx = TrapContext::new(user_stack, entrypoint, self.get_ptr());
        inner.syscall_times = [0; MAX_SYSCALL_NUM];
        inner.addr_space = ms;
        inner.state = TaskState::Ready;
    }

    pub fn get_ptr(&self) -> usize {
        self as *const _ as usize
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskInner> {
        self.inner.exclusive_access()
    }
}
