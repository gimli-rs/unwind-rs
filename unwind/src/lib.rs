#![cfg_attr(feature = "nightly", feature(unwind_attributes))]
#![cfg_attr(feature = "asm", feature(asm, naked_functions))]

extern crate gimli;
extern crate libc;
extern crate fallible_iterator;
#[macro_use] extern crate log;

use gimli::{UnwindSection, UnwindTable, UnwindTableRow, EhFrame, BaseAddresses, UninitializedUnwindContext, Pointer, Reader, EndianSlice, NativeEndian, CfaRule, RegisterRule, EhFrameHdr, ParsedEhFrameHdr, X86_64};
use fallible_iterator::FallibleIterator;

mod registers;
mod find_cfi;
mod range;
pub mod libunwind_shim;
pub mod glue;
pub use registers::Registers;
use find_cfi::EhRef;

pub struct StackFrames<'a> {
    unwinder: &'a mut DwarfUnwinder,
    registers: Registers,
    state: Option<(UnwindTableRow<StaticReader>, u64)>,
}

#[derive(Debug)]
pub struct StackFrame {
    personality: Option<u64>,
    lsda: Option<u64>,
    initial_address: u64,
}

impl StackFrame {
    pub fn personality(&self) -> Option<u64> {
        self.personality
    }

    pub fn lsda(&self) -> Option<u64> {
        self.lsda
    }

    pub fn initial_address(&self) -> u64 {
        self.initial_address
    }
}

pub trait Unwinder: Default {
    fn trace<F>(&mut self, f: F) where F: FnMut(&mut StackFrames);
}

type StaticReader = EndianSlice<'static, NativeEndian>;

struct ObjectRecord {
    er: EhRef,
    eh_frame_hdr: ParsedEhFrameHdr<StaticReader>,
    eh_frame: EhFrame<StaticReader>,
    bases: BaseAddresses,
}

pub struct DwarfUnwinder {
    cfi: Vec<ObjectRecord>,
    ctx: UninitializedUnwindContext<StaticReader>,
}

impl Default for DwarfUnwinder {
    fn default() -> DwarfUnwinder {
        let cfi = find_cfi::find_cfi_sections().into_iter().map(|er| {
            unsafe {
                // TODO: set_got()
                let bases = BaseAddresses::default()
                    .set_eh_frame_hdr(er.eh_frame_hdr.start)
                    .set_text(er.text.start);

                let eh_frame_hdr: &'static [u8] = std::slice::from_raw_parts(er.eh_frame_hdr.start as *const u8, er.eh_frame_hdr.len() as usize);

                let eh_frame_hdr = EhFrameHdr::new(eh_frame_hdr, NativeEndian).parse(&bases, 8).unwrap();

                let eh_frame_addr = deref_ptr(eh_frame_hdr.eh_frame_ptr());
                let eh_frame_sz = er.eh_frame_end.saturating_sub(eh_frame_addr);

                let eh_frame: &'static [u8] = std::slice::from_raw_parts(eh_frame_addr as *const u8, eh_frame_sz as usize);
                trace!("eh_frame at {:p} sz {:x}", eh_frame_addr as *const u8, eh_frame_sz);
                let eh_frame = EhFrame::new(eh_frame, NativeEndian);

                let bases = bases.set_eh_frame(eh_frame_addr);

                ObjectRecord { er, eh_frame_hdr, eh_frame, bases }
            }
        }).collect();

        DwarfUnwinder {
            cfi,
            ctx: UninitializedUnwindContext::new(),
        }
    }
}

impl Unwinder for DwarfUnwinder {
    fn trace<F>(&mut self, mut f: F) where F: FnMut(&mut StackFrames) {
        glue::registers(|registers| {
            let mut frames = StackFrames::new(self, registers);
            f(&mut frames)
        });
    }
}

struct UnwindInfo<R: Reader> {
    row: UnwindTableRow<R>,
    personality: Option<Pointer>,
    lsda: Option<Pointer>,
    initial_address: u64,
}

impl ObjectRecord {
    fn unwind_info_for_address(
        &self,
        ctx: &mut UninitializedUnwindContext<StaticReader>,
        address: u64,
    ) -> gimli::Result<UnwindInfo<StaticReader>> {
        let &ObjectRecord {
            ref eh_frame_hdr,
            ref eh_frame,
            ref bases,
            ..
        } = self;

        let fde = eh_frame_hdr.table().unwrap()
            .fde_for_address(eh_frame, bases, address, EhFrame::cie_from_offset)?;
        let mut result_row = None;
        {
            let mut table = UnwindTable::new(eh_frame, bases, ctx, &fde)?;
            while let Some(row) = table.next_row()? {
                if row.contains(address) {
                    result_row = Some(row.clone());
                    break;
                }
            }
        }

        match result_row {
            Some(row) => Ok(UnwindInfo {
                row,
                personality: fde.personality(),
                lsda: fde.lsda(),
                initial_address: fde.initial_address(),
            }),
            None => Err(gimli::Error::NoUnwindInfoForAddress)
        }
    }
}

unsafe fn deref_ptr(ptr: Pointer) -> u64 {
    match ptr {
        Pointer::Direct(x) => x,
        Pointer::Indirect(x) => *(x as *const u64),
    }
}


impl<'a> StackFrames<'a> {
    pub fn new(unwinder: &'a mut DwarfUnwinder, registers: Registers) -> Self {
        StackFrames {
            unwinder,
            registers,
            state: None,
        }
    }

    pub fn registers(&mut self) -> &mut Registers {
        &mut self.registers
    }
}

impl<'a> FallibleIterator for StackFrames<'a> {
    type Item = StackFrame;
    type Error = gimli::Error;

    fn next(&mut self) -> Result<Option<StackFrame>, Self::Error> {
        let registers = &mut self.registers;

        if let Some((row, cfa)) = self.state.take() {
            let mut newregs = registers.clone();
            newregs[X86_64::RA] = None;
            for &(reg, ref rule) in row.registers() {
                trace!("rule {:?} {:?}", reg, rule);
                assert!(reg != X86_64::RSP); // stack = cfa
                newregs[reg] = match *rule {
                    RegisterRule::Undefined => unreachable!(), // registers[reg],
                    RegisterRule::SameValue => Some(registers[reg].unwrap()), // not sure why this exists
                    RegisterRule::Register(r) => registers[r],
                    RegisterRule::Offset(n) => Some(unsafe { *((cfa.wrapping_add(n as u64)) as *const u64) }),
                    RegisterRule::ValOffset(n) => Some(cfa.wrapping_add(n as u64)),
                    RegisterRule::Expression(_) => unimplemented!(),
                    RegisterRule::ValExpression(_) => unimplemented!(),
                    RegisterRule::Architectural => unreachable!(),
                };
            }
            newregs[7] = Some(cfa);

            *registers = newregs;
            trace!("registers:{:?}", registers);
        }


        if let Some(mut caller) = registers[X86_64::RA] {
            caller -= 1; // THIS IS NECESSARY
            debug!("caller is 0x{:x}", caller);

            let rec = self.unwinder.cfi.iter().filter(|x| x.er.text.contains(caller)).next().ok_or(gimli::Error::NoUnwindInfoForAddress)?;

            let UnwindInfo { row, personality, lsda, initial_address } = rec.unwind_info_for_address(&mut self.unwinder.ctx, caller)?;

            trace!("ok: {:?} (0x{:x} - 0x{:x})", row.cfa(), row.start_address(), row.end_address());
            let cfa = match *row.cfa() {
                CfaRule::RegisterAndOffset { register, offset } =>
                    registers[register].unwrap().wrapping_add(offset as u64),
                _ => unimplemented!(),
            };
            trace!("cfa is 0x{:x}", cfa);

            self.state = Some((row, cfa));

            Ok(Some(StackFrame {
                personality: personality.map(|x| unsafe { deref_ptr(x) }),
                lsda: lsda.map(|x| unsafe { deref_ptr(x) }),
                initial_address,
            }))
        } else {
            Ok(None)
        }
    }
}
