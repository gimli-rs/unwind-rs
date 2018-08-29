use range::AddrRange;
use super::EhRef;

extern "C" {
    static __text_start: usize;
    static __text_end: usize;
    static __ehframehdr_start: usize;
    static __ehframehdr_end: usize;
    static __ehframe_end: usize;
}

pub fn find_cfi_sections() -> Vec<EhRef> {
    let mut cfi: Vec<EhRef> = Vec::new();
    unsafe {
        // Safety: None of those are actual accesses - we only get the address
        // of those values.
        let text_start = &__text_start as *const _ as u64;
        let text_end = &__text_end as *const _ as u64;
        let cfi_start = &__ehframehdr_start as *const _ as u64;
        let cfi_end = &__ehframehdr_end as *const _ as u64;
        let ehframe_end = &__ehframe_end as *const _ as u64;

        cfi.push(EhRef {
            obj_base: 0,
            text: AddrRange { start: text_start, end: text_end },
            cfi: AddrRange { start: cfi_start, end: cfi_end },
            ehframe_end,
        });
    }
    trace!("CFI sections: {:?}", cfi);
    cfi
}
