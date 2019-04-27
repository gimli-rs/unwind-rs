extern crate findshlibs;
extern crate object;
extern crate memmap;

use libc::{c_void, c_int, c_char};
use std::ffi::CStr;
use std::{slice, mem, cmp};
use range::AddrRange;
use super::EhRef;

use self::findshlibs::{SharedLibrary, Segment};
use self::object::{Object, ObjectSection};

extern "C" {
    #[link_name = "\x01segment$start$__TEXT"]
    static START_TEXT: usize;
    #[link_name = "\x01segment$end$__TEXT"]
    static STOP_TEXT: usize;

    #[link_name = "\x01section$start$__DATA$__eh_frame"]
    static START_EHFRAME: usize;
    #[link_name = "\x01section$end$__DATA$__eh_frame"]
    static STOP_EHFRAME: usize;
}

pub fn find_cfi_sections() -> Vec<EhRef> {
    let mut cfi: Vec<EhRef> = Vec::new();

    findshlibs::TargetSharedLibrary::each(|shlib| {
        let text_seg = shlib.segments().find(|seg| seg.name() == CStr::from_bytes_with_nul(b"__TEXT\0").unwrap()).expect("No code in library???");

        let text = AddrRange {
            start: text_seg.actual_virtual_memory_address(shlib).0 as u64,
            end: text_seg.actual_virtual_memory_address(shlib).0 as u64 + text_seg.len() as u64,
        };

        let file_bytes = unsafe { memmap::Mmap::map(&std::fs::File::open(shlib.name().to_str().unwrap()).unwrap()).unwrap() };

        let object = match object::MachOFile::parse(&*file_bytes) {
            Ok(object) => object,
            Err(err) => {
                if &file_bytes[0..4] == &[0xca, 0xfe, 0xba, 0xbe] {
                    //println!("can't parse fat mach-O file");
                    return;
                }

                println!("{}", shlib.name().to_str().unwrap());
                println!("{:?}", err);
                return;
            }
        };

        // FIXME fix memory leak
        let eh_frame: &'static [u8] = Box::leak(object.section_data_by_name(".eh_frame").unwrap().into_owned().into_boxed_slice());

        cfi.push(EhRef::WithoutHeader {
            text,
            eh_frame: AddrRange { start: eh_frame.as_ptr() as u64, end: eh_frame.as_ptr() as u64 + eh_frame.len() as u64 },
        });
    });

    println!("{:#?}", cfi);

    trace!("CFI sections: {:?}", cfi);
    cfi
}
