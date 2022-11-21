use core::fmt::Display;

use riscv::register::sstatus::{self, Sstatus, SPP};

use crate::{
    config::{PAGE_SIZE, TRAMPOLINE},
    mm::{VirtAddr, KERNEL_SPACE},
};

use super::handler::trap_handler;

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,  // 保存内核地址空间的token.
    pub kernel_sp: usize,    // 内核栈栈顶的虚拟地址.
    pub trap_handler: usize, // trap handler 入口点虚拟地址.
}

impl TrapContext {
    pub fn init(&mut self, user_stack: usize, entry_point: usize, kernel_stack: usize) {
        self.sepc = entry_point;
        self.sstatus = {
            let mut sstatus = sstatus::read();
            sstatus.set_spp(SPP::User);
            sstatus
        };
        self.kernel_satp = KERNEL_SPACE.lock().token();
        self.kernel_sp = kernel_stack;
        self.trap_handler = trap_handler as usize;

        log::debug!("TrapContext::new, set ctx.x2=0x{:x}", user_stack);
        self.x[2] = user_stack
    }

    pub fn reg_a(&self, n: usize) -> usize {
        self.x[10 + n]
    }

    pub fn set_reg_a(&mut self, n: usize, v: usize) {
        self.x[10 + n] = v
    }

    pub fn get_ptr(&mut self) -> usize {
        self as *mut TrapContext as usize
    }

    pub fn get_user_ptr(&mut self) -> usize {
        let offset = VirtAddr::from(self.get_ptr()).page_offset();
        TRAMPOLINE - PAGE_SIZE + offset
    }
}

impl Display for TrapContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "TrapContext {{x: {:x?},\n sstatus: 0x{:x},\n sepc: 0x{:x},\n kernel_satp: 0x{:x},\n kernel_sp:0x{:x},\n trap_handler:0x{:x}}}",
            self.x,
            self.sstatus.bits(),
            self.sepc,
            self.kernel_satp,
            self.kernel_sp,
            self.trap_handler
        ))
    }
}
