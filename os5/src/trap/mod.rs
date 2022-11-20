mod context;
mod handler;
mod restore;

use riscv::register::sie;
use riscv::register::{stvec, utvec::TrapMode};

pub use {context::TrapContext, restore::restore};

core::arch::global_asm!(include_str!("trap.S"));
extern "C" {
    fn __alltraps() -> !;
    fn __restore(user_ctx: usize, user_token: usize) -> !;
}

pub fn init() {
    unsafe { stvec::write(__alltraps as usize, TrapMode::Direct) }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
