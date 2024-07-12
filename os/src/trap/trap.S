.altmacro
.macro STORE_REGISTER n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_REGISTER n
    ld x\n, \n*8(sp)
.endm
    .section .text.trap
    .global _user_trap_entry
    .global _user_trap_return
    .global _kernel_trap_entry
    .global _kernel_trap_return
    .align 2
_user_trap_entry:
    #exchange stack, sscratch->user_stack, sp -> *Trapcontext in userspace
    csrrw sp, sscratch, sp
    #store x1(double words)
    sd x1, 1*8(sp)
    #skip x2, store x3~x31
    .set n, 3
    .rept 29
    STORE_REGISTER %n
    .set n, n+1
    .endr
    #now save sepc & sstatus
    csrr x5, sepc
    csrr x6, sstatus
    sd x5, 32*8(sp)
    sd x6, 33*8(sp)
    #save user_stack
    csrr x5, sscratch
    sd x5, 2*8(sp)
    #read *Trapcontext in kernelspace first
    ld a4, 38*8(sp)
    #read kernel stack
    ld a5, 34*8(sp)
    #read handler_addr
    ld a7, 37*8(sp)
    # now read kernel stap and switch to kernel space
    ld a6, 35*8(sp)
    csrw satp, a6
    sfence.vma
    #move to kernel stack
    mv sp, a5
    #set a0 for handler
    mv a0, a4
    jr a7

_user_trap_return:
    #read context addr in userspace, a0 is *Trapcontext in kernelspace
    ld x6, 38*8(a0)
    #store context addr in kernel space to Trapcontext
    sd a0, 38*8(a0)
    #read user stap
    ld x5, 36*8(a0)
    #switch to userspace
    csrw satp, x5
    sfence.vma
    #move sp -> *Trapcontext in userspace, then we can restore registers
    mv sp, x6
    #recover x1(double words)
    ld x1, 1*8(sp)
    #skip x2, recover x3~x31
    .set n, 3
    .rept 29
    LOAD_REGISTER %n
    .set n, n+1
    .endr
    #recover sepc & sstatus
    ld x5, 32*8(sp)
    ld x6, 33*8(sp)
    csrw sepc, x5
    csrw sstatus, x6
    #recover user_stack
    ld x5, 2*8(sp)
    csrw sscratch, x5
    #switch to user stack, sscratch -> *Trapcontext
    csrrw sp, sscratch, sp
    #return from scall
    sret

    .align 2
    #we did not know when the irq signal come, so all registers
    #should be restored
_kernel_trap_entry:
    addi sp, sp, -34*8
    #store common registers, x0(zero), x2(sp), x4(thread pointer) are ignored
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
    STORE_REGISTER %n
    .set n, n+1
    .endr
    #now save spec & sstatus
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    #get trap handler addr from sscratch
    csrr t2, sscratch
    jalr t2

_kernel_trap_return:
    #recover spec & sstatus first
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    #recover registers
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
    LOAD_REGISTER %n
    .set n, n+1
    .endr
    #recover sp
    addi sp, sp, 34*8
    sret