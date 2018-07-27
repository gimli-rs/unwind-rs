use super::{UnwindPayload, StackFrames};
use registers::{Registers, DwarfRegister};

#[allow(improper_ctypes)] // trampoline just forwards the ptr
extern "C" {
    #[cfg(not(feature = "nightly"))]
    pub fn unwind_trampoline(payload: *mut UnwindPayload);
    #[cfg(not(feature = "nightly"))]
    fn unwind_lander(regs: *const LandingRegisters);
}

#[cfg(feature = "nightly")]
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

#[cfg(feature = "nightly")]
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
    registers[DwarfRegister::Rbx] = Some(saved_regs.rbx);
    registers[DwarfRegister::Rbp] = Some(saved_regs.rbp);
    registers[DwarfRegister::SP] = Some(stack + 8);
    registers[DwarfRegister::R12] = Some(saved_regs.r12);
    registers[DwarfRegister::R13] = Some(saved_regs.r13);
    registers[DwarfRegister::R14] = Some(saved_regs.r14);
    registers[DwarfRegister::R15] = Some(saved_regs.r15);
    registers[DwarfRegister::IP] = Some(*(stack as *const u64));

    let mut frames = StackFrames {
        unwinder: payload.unwinder,
        registers,
        state: None,
    };

    (payload.tracer)(&mut frames);
}

pub unsafe fn land(regs: &Registers) {
    let mut lr = LandingRegisters {
        rax: regs[DwarfRegister::Rax].unwrap_or(0),
        rbx: regs[DwarfRegister::Rbx].unwrap_or(0),
        rcx: regs[DwarfRegister::Rcx].unwrap_or(0),
        rdx: regs[DwarfRegister::Rdx].unwrap_or(0),
        rdi: regs[DwarfRegister::Rdi].unwrap_or(0),
        rsi: regs[DwarfRegister::Rsi].unwrap_or(0),
        rbp: regs[DwarfRegister::Rbp].unwrap_or(0),
        r8:  regs[DwarfRegister::R8 ].unwrap_or(0),
        r9:  regs[DwarfRegister::R9 ].unwrap_or(0),
        r10: regs[DwarfRegister::R10].unwrap_or(0),
        r11: regs[DwarfRegister::R11].unwrap_or(0),
        r12: regs[DwarfRegister::R12].unwrap_or(0),
        r13: regs[DwarfRegister::R13].unwrap_or(0),
        r14: regs[DwarfRegister::R14].unwrap_or(0),
        r15: regs[DwarfRegister::R15].unwrap_or(0),
        rsp: regs[DwarfRegister::SP].unwrap(),
    };
    lr.rsp -= 8;
    *(lr.rsp as *mut u64) = regs[DwarfRegister::IP].unwrap();
    unwind_lander(&lr);
}
