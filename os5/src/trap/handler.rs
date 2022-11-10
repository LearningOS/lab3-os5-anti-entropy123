use crate::{
    syscall::{self, sys_exit},
    task::{run_next_task, Task, TaskState},
    timer::set_next_trigger,
    trap::restore,
};
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    stval,
};

#[no_mangle]
pub fn trap_handler(ctx: &mut Task) -> ! {
    let inner = ctx.inner_exclusive_access();
    let trap_ctx = &inner.trap_ctx;
    log::debug!("task_{} trap_handler, task.trap_ctx={}", ctx.id, trap_ctx);
    drop(inner);
    let scause = scause::read();
    let stval = stval::read();

    log::info!(
        "task_{} scause={:?}, stval=0x{:x}",
        ctx.id,
        scause.cause(),
        stval
    );
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            syscall::syscall_handler(ctx);
            let mut inner = ctx.inner_exclusive_access();
            inner.set_state(TaskState::Ready);
            inner.trap_ctx.sepc += 4;
            restore(ctx.get_ptr());
        }
        Trap::Exception(Exception::LoadPageFault) | Trap::Exception(Exception::StorePageFault) => {
            log::info!("page fault, try to access virtual address 0x{:x}", stval);
            sys_exit(ctx);
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) => {
            log::error!("memory access fault, core dump");
            sys_exit(ctx);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            log::error!("illegal instruction, core dump");
            sys_exit(ctx);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            log::info!("Timer interrupt.");
            set_next_trigger();
            let mut inner = ctx.inner_exclusive_access();
            inner.set_state(TaskState::Ready);
            run_next_task();
        }
        _ => {
            unimplemented!()
        }
    }
}
