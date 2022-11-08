mod context;
mod handler;

use riscv::register::sie;
use riscv::register::{stvec, utvec::TrapMode};

use crate::config::TRAMPOLINE;
use crate::mm::{VirtAddr, VirtPageNum, KERNEL_SPACE};
use crate::task::Task;
pub use context::TrapContext;

core::arch::global_asm!(include_str!("trap.S"));
extern "C" {
    fn __alltraps() -> !;
    fn __restore(user_ctx: usize, user_token: usize) -> !;
}

pub fn restore(ctx: usize) -> ! {
    log::debug!("try to get ctx addr by raw pointer, ctx_addr=0x{:x}", ctx);
    let task_ctx = unsafe { &*(ctx as *const Task) };
    let (user_trapctx, user_pt_token) = (task_ctx.get_user_ptr(), task_ctx.addr_space.token());
    log::debug!(
        "restore_from_trapctx, ctx={}, user_trapcontext_ptr=0x{:x}, user_pagetable_token=0x{:x}",
        task_ctx.trap_ctx,
        user_trapctx,
        user_pt_token
    );
    let restore_va = VirtAddr::from(TRAMPOLINE + VirtAddr::from(__restore as usize).page_offset());
    let trampoline = KERNEL_SPACE
        .lock()
        .translate(VirtPageNum::from(restore_va.floor()))
        .expect("should has the map");
    log::debug!(
        "__restore(trampoline) VA=0x{:x}, locate in page table entry=0x{:x}",
        restore_va.0,
        trampoline.bits
    );
    assert!(trampoline.is_valid());
    assert!(trampoline.executable());
    assert!(trampoline.readable());
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
        core::arch::asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va.0,
            in("a0") user_trapctx,
            in("a1") user_pt_token,
            options(noreturn)
        );
    }
}

pub fn init() {
    unsafe { stvec::write(__alltraps as usize, TrapMode::Direct) }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
