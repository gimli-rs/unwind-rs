.global unwind_trampoline
unwind_trampoline:
.cfi_startproc
movq %rsp, %rsi
.cfi_def_cfa rsi, 8
pushq %rbp
.cfi_offset rbp, -16
pushq %rbx
pushq %r12
pushq %r13
pushq %r14
pushq %r15
movq %rsp, %rdx
subq $0x08, %rsp
.cfi_def_cfa rsp, 0x40
call unwind_recorder
addq $0x38, %rsp
.cfi_def_cfa rsp, 8
ret
.cfi_endproc

.global unwind_lander
unwind_lander:
movq %rdi, %rsp
popq %rax
popq %rbx
popq %rcx
popq %rdx
popq %rdi
popq %rsi
popq %rbp
popq %r8
popq %r9
popq %r10
popq %r11
popq %r12
popq %r13
popq %r14
popq %r15
movq 0(%rsp), %rsp
ret /* HYPERSPACE JUMP :D */
