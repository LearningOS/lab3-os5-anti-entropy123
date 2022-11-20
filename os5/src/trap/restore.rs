use alloc::sync::Arc;
use riscv::register::stvec;
use riscv::register::utvec::TrapMode;

use crate::config::TRAMPOLINE;
use crate::mm::{VirtAddr, VirtPageNum, KERNEL_SPACE};
use crate::task::Task;

extern "C" {
    fn __restore(user_ctx: usize, user_token: usize) -> !;
}

pub fn restore(task_ctx: Arc<Task>) -> ! {
    let (user_trapctx, user_pt_token) = {
        let inner = task_ctx.inner_exclusive_access();
        let vals = (
            inner.trap_context().get_user_ptr(),
            inner.addr_space.token(),
        );
        log::trace!(
            "restore_from_trapctx, task_pid={}, trap_ctx={}, user_trapcontext_ptr=0x{:x}, user_pagetable_token=0x{:x}",
            task_ctx.pid,
            inner.trap_context(),
            vals.0,
            vals.1
        );
        vals
    };

    let restore_va = VirtAddr::from(TRAMPOLINE + VirtAddr::from(__restore as usize).page_offset());
    {
        let trampoline = {
            let trampoline = KERNEL_SPACE
                .lock()
                .translate(VirtPageNum::from(restore_va.floor()))
                .expect("should has the map");
            log::debug!(
                "__restore(trampoline) VA=0x{:x}, locate in page table entry=0x{:x}",
                restore_va.0,
                trampoline.bits
            );
            trampoline
        };
        assert!(trampoline.is_valid());
        assert!(trampoline.executable());
        assert!(trampoline.readable());

        let trampoline = {
            let restore_uva =
                VirtAddr::from(TRAMPOLINE + VirtAddr::from(__restore as usize).page_offset());
            let trampoline = task_ctx
                .inner_exclusive_access()
                .addr_space
                .translate(VirtPageNum::from(restore_uva.floor()))
                .expect("should has the map");
            log::debug!(
                "__restore(trampoline) VA=0x{:x}, locate in user page table entry=0x{:x}",
                restore_uva.0,
                trampoline.bits
            );
            trampoline
        };
        assert!(trampoline.is_valid());
        assert!(trampoline.executable());
        assert!(trampoline.readable());
    }

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
