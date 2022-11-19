// base
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;
pub const CLOCK_FREQ: usize = 12500000;

// kernel space config
pub const KERNEL_STACK_PAGE_NUM: usize = 20;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * KERNEL_STACK_PAGE_NUM;
pub const KERNEL_HEAP_SIZE: usize = PAGE_SIZE * 512;
pub const MEMORY_END: usize = 0x88000000;

// syscall/user config
pub const MAX_APP_NUM: usize = 10;
pub const MAX_SYSCALL_NUM: usize = 500;
pub const BIG_STRIDE: usize = usize::MAX;

// user space config
pub const USER_STACK_PAGE_NUM: usize = 20;
pub const USER_STACK_SIZE: usize = 4096 * USER_STACK_PAGE_NUM;
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
