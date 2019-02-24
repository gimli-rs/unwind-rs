use range::AddrRange;

#[derive(Debug)]
pub struct EhRef {
    pub obj_base: u64,
    pub text: AddrRange,
    pub eh_frame_hdr: AddrRange,
    pub eh_frame_end: u64,
}

#[cfg(unix)]
#[path = "ld.rs"]
mod imp;

#[cfg(not(unix))]
#[path = "baremetal.rs"]
mod imp;


pub use self::imp::find_cfi_sections;
