use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::ops::{Index, IndexMut};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Registers {
    registers: [Option<u64>; 32],
}

impl Debug for Registers {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        for reg in &self.registers {
            match *reg {
                None => write!(fmt, " XXX")?,
                Some(x) => write!(fmt, " 0x{:x}", x)?,
            }
        }
        Ok(())
    }
}

impl Index<u8> for Registers {
    type Output = Option<u64>;

    fn index(&self, index: u8) -> &Option<u64> {
        &self.registers[index as usize]
    }
}

impl IndexMut<u8> for Registers {
    fn index_mut(&mut self, index: u8) -> &mut Option<u64> {
        &mut self.registers[index as usize]
    }
}

impl Index<DwarfRegisterAMD64> for Registers {
    type Output = Option<u64>;

    fn index(&self, reg: DwarfRegisterAMD64) -> &Option<u64> {
        &self[reg as u8]
    }
}

impl IndexMut<DwarfRegisterAMD64> for Registers {
    fn index_mut(&mut self, reg: DwarfRegisterAMD64) -> &mut Option<u64> {
        &mut self[reg as u8]
    }
}

impl Index<DwarfRegisterAArch64> for Registers {
    type Output = Option<u64>;

    fn index(&self, reg: DwarfRegisterAArch64) -> &Option<u64> {
        &self[reg as u8]
    }
}

impl IndexMut<DwarfRegisterAArch64> for Registers {
    fn index_mut(&mut self, reg: DwarfRegisterAArch64) -> &mut Option<u64> {
        &mut self[reg as u8]
    }
}

pub enum DwarfRegisterAMD64 {
    SP = 7,
    IP = 16,
    
    Rax = 0,
    Rbx = 3,
    Rcx = 2,
    Rdx = 1,
    Rdi = 5,
    Rsi = 4,
    Rbp = 6,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

pub enum DwarfRegisterAArch64 {
    X0 = 0,
    X1 = 1,
    X2 = 2,
    X3 = 3,
    X4 = 4,
    X5 = 5,
    X6 = 6,
    X7 = 7,
    X8 = 8,
    X9 = 9,
    X10 = 10,
    X11 = 11,
    X12 = 12,
    X13 = 13,
    X14 = 14,
    X15 = 15,
    X16 = 16,
    X17 = 17,
    X18 = 18,
    X19 = 19,
    X20 = 20,
    X21 = 21,
    X22 = 22,
    X23 = 23,
    X24 = 24,
    X25 = 25,
    X26 = 26,
    X27 = 27,
    X28 = 28,
    X29 = 29, // Frame Pointer
    IP = 30, // Link register, x30, IP is restored in it?
    SP = 31,

    // ELR_mode
    // Vector regs
}

#[cfg(target_arch = "x86_64")]
pub use self::DwarfRegisterAMD64 as DwarfRegister;

#[cfg(target_arch = "aarch64")]
pub use self::DwarfRegisterAArch64 as DwarfRegister;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compiler_error!("Unsupported architecture");
