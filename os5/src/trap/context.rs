use core::fmt::Display;

use riscv::register::sstatus::{self, Sstatus, SPP};

use crate::mm::KERNEL_SPACE;

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
    pub fn app_init_context(user_stack: usize, entry_point: usize, kernel_stack: usize) -> Self {
        let mut ctx = TrapContext {
            x: [0; 32],
            sepc: entry_point,
            sstatus: {
                let mut sstatus = sstatus::read();
                sstatus.set_spp(SPP::User);
                sstatus
            },
            kernel_satp: KERNEL_SPACE.lock().token(),
            kernel_sp: kernel_stack,
            trap_handler: trap_handler as usize,
        };
        ctx.x[2] = user_stack;
        ctx
    }

    pub fn reg_a(&self, n: usize) -> usize {
        self.x[10 + n]
    }

    pub fn set_reg_a(&mut self, n: usize, v: usize) {
        self.x[10 + n] = v
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
