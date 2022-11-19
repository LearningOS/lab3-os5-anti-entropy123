use core::cell::RefMut;

use alloc::sync::Arc;
use lazy_static::lazy_static;

use super::Task;
use crate::sync::UPSafeCell;

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}
pub struct Processor {
    pub cur_task: Option<Arc<Task>>,
}

impl Processor {
    fn new() -> Self {
        Self { cur_task: None }
    }

    pub fn pop_task(&mut self) -> Option<Arc<Task>> {
        let cur_task = match &self.cur_task {
            None => return None,
            Some(task) => Arc::clone(task),
        };
        self.cur_task = None;
        Some(cur_task)
    }
}

pub fn processor_inner() -> RefMut<'static, Processor> {
    PROCESSOR.exclusive_access()
}
