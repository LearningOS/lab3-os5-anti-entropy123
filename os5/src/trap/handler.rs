use crate::{
    syscall::{self, sys_exit},
    task::{pop_cur_task, run_task, switch_task, weak_cur_task, Task, TaskState},
    timer::set_next_trigger,
};
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    stval,
};

#[no_mangle]
pub fn trap_handler() -> ! {
    super::init();
    let scause = scause::read();
    let stval = stval::read();
    let weak_task = weak_cur_task().expect("still not run user task?");
    {
        let task = Task::from_weak(&weak_task);
        let inner = task.inner_exclusive_access();
        let trap_ctx = inner.trap_context();
        log::debug!("task_{} trap_handler, task.trap_ctx={}", task.pid, trap_ctx);
        log::info!(
            "task_{} scause={:?}, stval=0x{:x}",
            task.pid,
            scause.cause(),
            stval
        );
    };

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            {
                Task::from_weak(&weak_task)
                    .inner_exclusive_access()
                    .trap_context()
                    .sepc += 4;
            };
            syscall::syscall_handler(&weak_task);
            {
                Task::from_weak(&weak_task)
                    .inner_exclusive_access()
                    .set_state(TaskState::Ready);
            };
            run_task(pop_cur_task().unwrap());
        }
        Trap::Exception(Exception::LoadPageFault) | Trap::Exception(Exception::StorePageFault) => {
            log::info!("page fault, try to access virtual address 0x{:x}", stval);
            sys_exit(pop_cur_task().unwrap(), 1);
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) => {
            log::error!("memory access fault, core dump");
            sys_exit(pop_cur_task().unwrap(), 1);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            log::error!("illegal instruction, core dump");
            sys_exit(pop_cur_task().unwrap(), 1);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            log::info!("Timer interrupt.");
            set_next_trigger();
            {
                let task = Task::from_weak(&weak_task);
                let mut inner = task.inner_exclusive_access();
                inner.set_state(TaskState::Ready);
            }
            switch_task(pop_cur_task().unwrap());
        }
        _ => {
            unimplemented!()
        }
    }
}
