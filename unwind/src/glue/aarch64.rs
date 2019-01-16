use {UnwindPayload, StackFrames};
use registers::{Registers, DwarfRegisterAArch64};

#[allow(improper_ctypes)] // trampoline just forwards the ptr
extern "C" {
    pub fn unwind_trampoline(payload: *mut UnwindPayload);
    fn unwind_lander(regs: *const LandingRegisters);
}

#[cfg(feature = "nightly")]
global_asm! {
r#"
.global unwind_trampoline
unwind_trampoline:
.cfi_startproc
     mov x1, sp
     sub sp, sp, 0xA0
     .cfi_adjust_cfa_offset 0xA0
     stp x19, x20, [sp, #0x00]
     stp x21, x22, [sp, #0x10]
     stp x23, x24, [sp, #0x20]
     stp x25, x26, [sp, #0x30]
     stp x27, x28, [sp, #0x40]
     stp x29, lr,  [sp, #0x50]
     .cfi_rel_offset lr, 0x58
     stp d8,  d9,  [sp, #0x60]
     stp d10, d11, [sp, #0x70]
     stp d12, d13, [sp, #0x80]
     stp d14, d15, [sp, #0x90]
     mov x2, sp
     bl unwind_recorder
     ldr lr, [sp, #0x58]
     .cfi_restore lr
     add sp, sp, 0xA0
     .cfi_adjust_cfa_offset -0xA0
     ret
.cfi_endproc

.global unwind_lander
unwind_lander:
     ldp x2,  x3,  [x0, #0x010]
     ldp x4,  x5,  [x0, #0x020]
     ldp x6,  x7,  [x0, #0x030]
     ldp x8,  x9,  [x0, #0x040]
     ldp x10, x11, [x0, #0x050]
     ldp x12, x13, [x0, #0x060]
     ldp x14, x15, [x0, #0x070]
     ldp x16, x17, [x0, #0x080]
     ldp x18, x19, [x0, #0x090]
     ldp x20, x21, [x0, #0x0A0]
     ldp x22, x23, [x0, #0x0B0]
     ldp x24, x25, [x0, #0x0C0]
     ldp x26, x27, [x0, #0x0D0]
     ldp x28, x29, [x0, #0x0E0]
     ldp x30, x1,  [x0, #0x0F0]
     mov sp, x1

     ldp d0,  d1,  [x0, #0x100]
     ldp d2,  d3,  [x0, #0x110]
     ldp d4,  d5,  [x0, #0x120]
     ldp d6,  d7,  [x0, #0x130]
     ldp d8,  d9,  [x0, #0x140]
     ldp d10, d11, [x0, #0x150]
     ldp d12, d13, [x0, #0x160]
     ldp d14, d15, [x0, #0x170]
     ldp d16, d17, [x0, #0x180]
     ldp d18, d19, [x0, #0x190]
     ldp d20, d21, [x0, #0x1A0]
     ldp d22, d23, [x0, #0x1B0]
     ldp d24, d25, [x0, #0x1C0]
     ldp d26, d27, [x0, #0x1D0]
     ldp d28, d29, [x0, #0x1E0]
     ldp d30, d31, [x0, #0x1F0]

     ldp x0,  x1,  [x0, #0x000]
     ret x30 // HYPERSPACE JUMP :D
"#
}

#[repr(C)]
struct LandingRegisters {
    r: [u64; 29], // x0-x28
    fp: u64,      // x29, Frame Pointer
    lr: u64,      // x30, Link Register
    sp: u64,      // x31, Stack Pointer
    vector_half: [u64; 32], // d0-d31
}

// TODO: Doc hidden
#[repr(C)]
pub struct SavedRegs {
    r: [u64; 11], // x19-x29
    lr: u64,
    vector_half: [u64; 8], // d8-d15
}

// TODO: doc hidden
#[no_mangle]
pub unsafe extern "C" fn unwind_recorder(payload: *mut UnwindPayload, stack: u64, saved_regs: *mut SavedRegs) {
    let payload = &mut *payload;
    let saved_regs = &*saved_regs;

    let mut registers = Registers::default();
    for (regnum, v) in saved_regs.r.iter().enumerate() {
        registers[DwarfRegisterAArch64::X19 as u8 + regnum as u8] = Some(*v);
    }
    for (regnum, v) in saved_regs.vector_half.iter().enumerate() {
        registers[DwarfRegisterAArch64::V8 as u8 + regnum as u8] = Some(*v);
    }
    registers[DwarfRegisterAArch64::SP] = Some(stack);
    registers[DwarfRegisterAArch64::IP] = Some(saved_regs.lr);

    let mut frames = StackFrames {
        unwinder: payload.unwinder,
        registers,
        state: None,
    };

    (payload.tracer)(&mut frames);
}

pub unsafe fn land(regs: &Registers) {
    let mut lr = LandingRegisters {
        r: [0; 29],
        fp: regs[DwarfRegisterAArch64::X29].unwrap_or(0),
        lr: regs[DwarfRegisterAArch64::IP].unwrap_or(0),
        sp: regs[DwarfRegisterAArch64::SP].unwrap_or(0),
        vector_half: [0; 32]
    };
    for (i, v) in lr.r.iter_mut().enumerate() {
        *v = regs[i as u8].unwrap_or(0);
    }

    for (i, v) in lr.vector_half.iter_mut().enumerate() {
        *v = regs[DwarfRegisterAArch64::V0 as u8 + i as u8].unwrap_or(0);
    }
    unwind_lander(&lr);
}
