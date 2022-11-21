use core::fmt::Display;
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct PidHandle(pub usize);

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::from(PidAllocator::new());
}

struct PidAllocator {
    current: usize,
}

impl PidAllocator {
    fn new() -> Self {
        PidAllocator { current: 0 }
    }

    fn alloc(&mut self) -> PidHandle {
        self.current += 1;
        PidHandle(self.current)
    }
    #[allow(dead_code)]
    fn dealloc(&mut self) -> PidHandle {
        PidHandle(0)
    }
}

impl Display for PidHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

pub fn alloc_pid() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}
