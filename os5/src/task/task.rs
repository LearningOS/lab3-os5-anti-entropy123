use crate::{
    config::*,
    loader::{get_app_elf, get_kernel_stack_phyaddr},
    mm::{MemorySet, PhysAddr, VirtAddr, VirtPageNum},
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
pub struct Task {
    pub trap_ctx: TrapContext,
    pub id: usize,
    state: TaskState,
    pub start_time_ms: usize,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub addr_space: MemorySet,
}

impl Task {
    pub fn from(&mut self, app_id: usize) {
        let (ms, user_stack, entrypoint) =
            MemorySet::from_elf(get_app_elf(app_id), get_kernel_stack_phyaddr(app_id));

        self.trap_ctx = TrapContext::app_init_context(user_stack, entrypoint, self.get_ptr());
        self.id = app_id;
        self.syscall_times = [0; MAX_SYSCALL_NUM];
        self.start_time_ms = 0;
        self.addr_space = ms;
        self.state = TaskState::Ready;
    }

    pub fn get_ptr(&self) -> usize {
        self as *const _ as usize
    }

    pub fn get_user_ptr(&self) -> usize {
        TRAMPOLINE - core::mem::size_of::<Task>()
    }

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
