.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm

.section .text.trampoline
.globl __alltraps
.globl __restore
.align 2
__alltraps:
    csrrw sp, sscratch, sp
    # sp -> kernel stack.
    # addi sp, sp, -34*8

    SAVE_GP 1
    .set n, 3
    .rept 29
        SAVE_GP %n
        .set n, n+1
    .endr

    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp) # ctx.sstatus <- user_sstatus
    sd t1, 33*8(sp) # ctx.sepc <- user_pc
    csrr t2, sscratch
    sd t2, 2*8(sp)  # ctx.x[2] <- user_stack
    # load kernel_satp into t0
    ld t0, 34*8(sp)
    # load trap_handler into t1
    ld t1, 36*8(sp)
    ld a0, 35*8(sp)
    mv sp, a0
    # move to kernel_sp
    csrw satp, t0
    sfence.vma
    jr t1

.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

# Rust function define: fn __restore(user_ctx: TrapContext, user_token: usize) -> ! ;
.align 2
__restore:
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0 
    mv sp, a0
    ld t0, 32*8(sp) // sstatus
    ld t1, 33*8(sp) // sepc
    
    csrw sstatus, t0
    csrw sepc, t1

    ld x1, 1*8(sp)
    .set n, 3
    .rept 29
        LOAD_GP %n
        .set n, n+1
    .endr
    ld sp, 2*8(sp)
    sret
