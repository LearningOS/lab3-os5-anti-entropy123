pub struct PidHandle(pub usize);

struct PidAllocator {
    current: usize,
}

impl PidAllocator {
    fn new() -> Self {
        PidAllocator { current: 0 }
    }

    fn alloc(&mut self) -> PidHandle {
        PidHandle(0)
    }
    fn dealloc(&mut self) -> PidHandle {
        PidHandle(0)
    }
}
