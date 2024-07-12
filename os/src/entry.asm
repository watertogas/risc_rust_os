    .section .text.entry
    .global _start
_start:
    la sp, kernel_stack_top
    call entry_os

    .section .bss.kstack
    .global kernel_stack_bottom
kernel_stack_bottom:
    .space 4096 * 16
    .global kernel_stack_top
kernel_stack_top: