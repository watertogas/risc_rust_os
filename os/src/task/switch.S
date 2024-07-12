    .section .text.text
    .global __switch_from
    .global __switch_to

.altmacro
.macro STORE_SREGISTER n
    sd s\n, \n*8(a0)
.endm
.macro LOAD_SREGISTER n
    ld s\n, \n*8(a0)
.endm

#switch to idle task; a0: in context, a1: idle context
#__switch_from(current_task_cx_ptr: *const TaskContext, idle_task_cx_ptr: *const Idlecontext);
__switch_from:
    #restore task context
    .set n, 0
    .rept 12
    STORE_SREGISTER %n
    .set n, n+1
    .endr
    #now save sp & ra
    sd ra, 12*8(a0)
    sd sp, 13*8(a0)
    ld ra, 8(a1)
    ld sp, (a1)
    ret
#__switch_to(dst_task_cx_ptr: *const TaskContext);
__switch_to:
    #recover task context
    .set n, 0
    .rept 12
    LOAD_SREGISTER %n
    .set n, n+1
    .endr
    #recover ra & ra
    ld ra, 12*8(a0)
    ld sp, 13*8(a0)
    ret