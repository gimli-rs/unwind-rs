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
