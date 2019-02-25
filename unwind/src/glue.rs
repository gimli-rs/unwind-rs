use gimli::X86_64;
use super::{UnwindPayload, StackFrames};
use registers::Registers;

#[allow(improper_ctypes)] // trampoline just forwards the ptr
extern "C" {
    #[cfg(not(feature = "asm"))]
    pub fn unwind_trampoline(payload: *mut UnwindPayload);
    #[cfg(not(feature = "asm"))]
    fn unwind_lander(regs: *const LandingRegisters);
}

#[cfg(feature = "asm")]
#[naked]
pub unsafe extern fn unwind_trampoline(_payload: *mut UnwindPayload) {
    asm!("
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
     subq 0x08, %rsp
     .cfi_def_cfa rsp, 0x40
     call unwind_recorder
     addq 0x38, %rsp
     .cfi_def_cfa rsp, 8
     ret
     ");
    ::std::hint::unreachable_unchecked();
}

#[cfg(feature = "asm")]
#[naked]
unsafe extern fn unwind_lander(_regs: *const LandingRegisters) {
    asm!("
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
     ret // HYPERSPACE JUMP :D
     ");
    ::std::hint::unreachable_unchecked();
}

#[repr(C)]
struct LandingRegisters {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rsp: u64,
    // rflags? cs,fs,gs?
}

#[repr(C)]
pub struct SavedRegs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
}

#[no_mangle]
pub unsafe extern "C" fn unwind_recorder(payload: *mut UnwindPayload, stack: u64, saved_regs: *mut SavedRegs) {
    let payload = &mut *payload;
    let saved_regs = &*saved_regs;

    let mut registers = Registers::default();
    registers[X86_64::RBX] = Some(saved_regs.rbx);
    registers[X86_64::RBP] = Some(saved_regs.rbp);
    registers[X86_64::RSP] = Some(stack + 8);
    registers[X86_64::R12] = Some(saved_regs.r12);
    registers[X86_64::R13] = Some(saved_regs.r13);
    registers[X86_64::R14] = Some(saved_regs.r14);
    registers[X86_64::R15] = Some(saved_regs.r15);
    registers[X86_64::RA] = Some(*(stack as *const u64));

    let mut frames = StackFrames {
        unwinder: payload.unwinder,
        registers,
        state: None,
    };

    (payload.tracer)(&mut frames);
}

pub unsafe fn land(regs: &Registers) {
    let mut lr = LandingRegisters {
        rax: regs[X86_64::RAX].unwrap_or(0),
        rbx: regs[X86_64::RBX].unwrap_or(0),
        rcx: regs[X86_64::RCX].unwrap_or(0),
        rdx: regs[X86_64::RDX].unwrap_or(0),
        rdi: regs[X86_64::RDI].unwrap_or(0),
        rsi: regs[X86_64::RSI].unwrap_or(0),
        rbp: regs[X86_64::RBP].unwrap_or(0),
        r8:  regs[X86_64::R8 ].unwrap_or(0),
        r9:  regs[X86_64::R9 ].unwrap_or(0),
        r10: regs[X86_64::R10].unwrap_or(0),
        r11: regs[X86_64::R11].unwrap_or(0),
        r12: regs[X86_64::R12].unwrap_or(0),
        r13: regs[X86_64::R13].unwrap_or(0),
        r14: regs[X86_64::R14].unwrap_or(0),
        r15: regs[X86_64::R15].unwrap_or(0),
        rsp: regs[X86_64::RSP].unwrap(),
    };
    lr.rsp -= 8;
    *(lr.rsp as *mut u64) = regs[X86_64::RA].unwrap();
    unwind_lander(&lr);
}
