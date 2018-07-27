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
