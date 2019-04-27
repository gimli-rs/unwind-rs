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
        let text = AddrRange {
            start: &__text_start as *const _ as u64,
            end: &__text_end as *const _ as u64,
        };
        let eh_frame_hdr = AddrRange {
            start: &__ehframehdr_start as *const _ as u64,
            end: &__ehframehdr_end as *const _ as u64,
        };
        let eh_frame_end = &__ehframe_end as *const _ as u64;

        cfi.push(EhRef::WithHeader {
            text,
            eh_frame_hdr,
            eh_frame_end,
        });
    }
    trace!("CFI sections: {:?}", cfi);
    cfi
}
