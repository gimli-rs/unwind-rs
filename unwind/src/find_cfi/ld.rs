use libc::{c_void, c_int, c_char};
use std::ffi::CStr;
use std::{slice, mem, cmp};
use range::AddrRange;
use super::EhRef;

#[repr(C)]
struct DlPhdrInfo {
    addr: u64,
    name: *const c_char,
    phdr: *const Phdr64,
    phnum: u16,
}

/*
#[repr(C)]
struct Phdr32 {
    type_: u32,
    offset: u32,
    vaddr: u32,
    paddr: u32,
    filesz: u32,
    memsz: u32,
    flags: u32,
    align: u32,
}
*/

#[repr(C)]
struct Phdr64 {
    type_: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    paddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

const PT_GNU_EH_FRAME: u32 = 0x6474e550;
const PT_LOAD: u32 = 1;

type PhdrCb = extern "C" fn(info: *const DlPhdrInfo, size: usize, data: *mut c_void) -> c_int;
extern "C" {
    fn dl_iterate_phdr(callback: PhdrCb, data: *mut c_void) -> c_int;
}

extern "C" fn callback(info: *const DlPhdrInfo, size: usize, data: *mut c_void) -> c_int {
    let data = data as *mut Vec<EhRef>;
    assert!(size >= mem::size_of::<DlPhdrInfo>());

    unsafe {
        let name = CStr::from_ptr((*info).name);
        trace!("{:?} at 0x{:x} with {} segments", name, (*info).addr, (*info).phnum);

        let phdr = slice::from_raw_parts((*info).phdr, (*info).phnum as usize);

        if let Some(text) = phdr.iter().filter(|x| x.type_ == PT_LOAD).next() {
            if let Some(eh_frame) = phdr.iter().filter(|x| x.type_ == PT_GNU_EH_FRAME).next() {
                let start_addr = (*info).addr + text.vaddr;
                let cfi_start = (*info).addr + eh_frame.vaddr;
                let max_vaddr = phdr.iter().filter(|x| x.type_ == PT_LOAD)
                    .fold(0, |vaddr, x| cmp::max(vaddr, x.vaddr + x.memsz));
                // This is an upper bound, not the exact address.
                let ehframe_end = (*info).addr + max_vaddr;
                (*data).push(EhRef {
                    obj_base: (*info).addr,
                    text: AddrRange { start: start_addr, end: start_addr + text.memsz },
                    cfi: AddrRange { start: cfi_start, end: cfi_start + eh_frame.memsz },
                    ehframe_end,
                });
            }
        }

        0
    }
}

pub fn find_cfi_sections() -> Vec<EhRef> {
    let mut cfi: Vec<EhRef> = Vec::new();
    unsafe { dl_iterate_phdr(callback, &mut cfi as *mut _ as *mut c_void) };
    trace!("CFI sections: {:?}", cfi);
    cfi
}
