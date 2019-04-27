#![cfg_attr(feature = "nightly", feature(unwind_attributes))]
#![cfg_attr(feature = "asm", feature(asm, naked_functions))]

extern crate gimli;
extern crate libc;
extern crate fallible_iterator;
#[macro_use] extern crate log;
extern crate backtrace;

use gimli::{UnwindSection, UnwindTable, UnwindTableRow, EhFrame, BaseAddresses, UninitializedUnwindContext, Pointer, Reader, EndianSlice, NativeEndian, CfaRule, RegisterRule, EhFrameHdr, ParsedEhFrameHdr, X86_64};
use fallible_iterator::FallibleIterator;

pub mod registers;
mod find_cfi;
mod range;
pub mod libunwind_shim;
pub mod glue;
use registers::Registers;
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
    eh_frame_hdr: Option<ParsedEhFrameHdr<StaticReader>>,
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
            match er {
                EhRef::WithHeader {
                    text,
                    eh_frame_hdr,
                    eh_frame_end,
                } => unsafe {
                    // TODO: set_got()
                    let bases = BaseAddresses::default()
                        .set_eh_frame_hdr(eh_frame_hdr.start)
                        .set_text(text.start);

                    let eh_frame_hdr: &'static [u8] = std::slice::from_raw_parts(eh_frame_hdr.start as *const u8, eh_frame_hdr.len() as usize);

                    let eh_frame_hdr = EhFrameHdr::new(eh_frame_hdr, NativeEndian).parse(&bases, 8).unwrap();

                    let eh_frame_addr = deref_ptr(eh_frame_hdr.eh_frame_ptr());
                    let eh_frame_sz = eh_frame_end.saturating_sub(eh_frame_addr);

                    let eh_frame: &'static [u8] = std::slice::from_raw_parts(eh_frame_addr as *const u8, eh_frame_sz as usize);
                    trace!("eh_frame at {:p} sz {:x}", eh_frame_addr as *const u8, eh_frame_sz);
                    let eh_frame = EhFrame::new(eh_frame, NativeEndian);

                    let bases = bases.set_eh_frame(eh_frame_addr);

                    ObjectRecord { er, eh_frame_hdr: Some(eh_frame_hdr), eh_frame, bases }
                }
                EhRef::WithoutHeader {
                    text,
                    eh_frame,
                } => unsafe {
                    // TODO: set_got()
                    let bases = BaseAddresses::default()
                        .set_text(text.start);

                    let eh_frame_addr = eh_frame.start as *const u8;
                    let eh_frame_sz = eh_frame.end as usize - eh_frame.start as usize;
                    let eh_frame: &'static [u8] = std::slice::from_raw_parts(eh_frame_addr, eh_frame_sz);
                    trace!("eh_frame at {:p} sz {:x}", eh_frame_addr as *const u8, eh_frame_sz);
                    let eh_frame = EhFrame::new(eh_frame, NativeEndian);

                    let bases = bases.set_eh_frame(eh_frame_addr as u64);

                    ObjectRecord { er, eh_frame_hdr: None, eh_frame, bases }
                }
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

        let fde;
        let mut result_row = None;
        if let Some(eh_frame_hdr) = eh_frame_hdr {
            fde = eh_frame_hdr.table().unwrap()
                .fde_for_address(eh_frame, bases, address, |eh_frame, bases, offset| eh_frame.cie_from_offset(bases, offset))?;

            {
                let mut table = UnwindTable::new(eh_frame, bases, ctx, &fde)?;
                while let Some(row) = table.next_row()? {
                    if row.contains(address) {
                        result_row = Some(row.clone());
                        break;
                    }
                }
            }
        } else {
            use gimli::UnwindSection;

            let mut entries = eh_frame.entries(bases);
            backtrace::resolve(address as *mut _, |s| {
                println!("address {:016x}: {:?}", address, s.name());
                let (text, eh_frame) = match self.er { EhRef::WithoutHeader { text, eh_frame, .. } => (text, eh_frame), _ => panic!() };
                println!("bases {:?}", bases);
                println!("text {:016x} .. {:016x}", text.start, text.end);
                println!("eh_frame {:016x} .. {:016x}", eh_frame.start, eh_frame.end);
            });
            while let Some(entry) = entries.next()? {
                match entry {
                    gimli::CieOrFde::Cie(_) => {}
                    gimli::CieOrFde::Fde(partial) => {
                        let fde = partial.parse(|eh_frame, bases, offset| eh_frame.cie_from_offset(bases, offset)).unwrap();
                        println!("fde {:016x} .. {:016x}", fde.initial_address(), fde.initial_address() + fde.len());
                        //println!("{:?}", fde);
                    }
                }
            }

            fde = eh_frame.fde_for_address(bases, address, |eh_frame, bases, offset| eh_frame.cie_from_offset(bases, offset)).unwrap();
            result_row = Some(eh_frame.unwind_info_for_address(
                bases,
                ctx,
                address,
                |eh_frame, bases, offset| eh_frame.cie_from_offset(bases, offset),
            )?);
        }

        match result_row {
            Some(row) => Ok(UnwindInfo {
                row,
                personality: fde.personality(),
                lsda: fde.lsda(),
                initial_address: fde.initial_address(),
            }),
            None => panic!(),
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

            println!("{:?}, {:?}", caller, self.unwinder.cfi.iter().map(|x| x.er.clone()).collect::<Vec<_>>());

            let rec = self.unwinder.cfi
                .iter()
                .filter(|x| match x.er {
                    EhRef::WithoutHeader { text, .. } | EhRef::WithHeader { text, .. } => text.contains(caller),
                })
                .next()
                .ok_or(gimli::Error::NoUnwindInfoForAddress)?;

            println!("found");

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
