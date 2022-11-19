use crate::{
    syscall::{self, sys_exit},
    task::{pop_task, run_task, switch_task, TaskState},
    timer::set_next_trigger,
};
use alloc::sync::Arc;
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    stval,
};

#[no_mangle]
pub fn trap_handler() -> ! {
    let task = pop_task().expect("still not run user task?");
    {
        let inner = task.inner_exclusive_access();
        let trap_ctx = inner.trap_context();
        log::debug!("task_{} trap_handler, task.trap_ctx={}", task.pid, trap_ctx);
    };
    let scause = scause::read();
    let stval = stval::read();

    log::info!(
        "task_{} scause={:?}, stval=0x{:x}",
        task.pid,
        scause.cause(),
        stval
    );
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            {
                task.inner_exclusive_access().trap_context().sepc += 4;
            };
            syscall::syscall_handler(Arc::clone(&task));
            {
                task.inner_exclusive_access().set_state(TaskState::Ready);
            };
            run_task(task);
        }
        Trap::Exception(Exception::LoadPageFault) | Trap::Exception(Exception::StorePageFault) => {
            log::info!("page fault, try to access virtual address 0x{:x}", stval);
            sys_exit(task);
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) => {
            log::error!("memory access fault, core dump");
            sys_exit(task);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            log::error!("illegal instruction, core dump");
            sys_exit(task);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            log::info!("Timer interrupt.");
            set_next_trigger();
            {
                let mut inner = task.inner_exclusive_access();
                inner.set_state(TaskState::Ready);
            }
            switch_task(task);
        }
        _ => {
            unimplemented!()
        }
    }
}
