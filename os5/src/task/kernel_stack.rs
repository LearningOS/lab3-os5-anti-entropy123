use crate::{
    config::*,
    mm::{MapPermission, VirtAddr, KERNEL_SPACE},
    task::PidHandle,
};

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    pub fn position(&self) -> (usize, usize) {
        kernel_stack_position(self.pid)
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        log::info!("drop kernel_stack, pid={}", self.pid);
        let (kernel_stack_bottom, _) = self.position();
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}

fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub fn alloc_kernel_stack(pid: PidHandle) -> KernelStack {
    log::info!("alloc_kernel_stack, pid={}", pid);
    let pid = pid.0;
    let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
    KERNEL_SPACE
        .lock()
        .insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        )
        .expect("map kernel stack failed!");
    log::info!(
        "alloc_kernel_stack finish, pid={}, stack_bottom=0x{:x}, stack_top=0x{:x}",
        pid,
        kernel_stack_bottom,
        kernel_stack_top
    );
    KernelStack { pid }
}
