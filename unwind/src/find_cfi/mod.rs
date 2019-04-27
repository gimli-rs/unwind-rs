use range::AddrRange;

#[derive(Debug, Clone)]
pub enum EhRef {
    WithHeader {
        text: AddrRange,
        eh_frame_hdr: AddrRange,
        eh_frame_end: u64,
    },
    WithoutHeader {
        text: AddrRange,
        eh_frame: AddrRange,
    },
}

#[cfg(all(unix, not(target_os = "macos")))]
#[path = "ld.rs"]
mod imp;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod imp;

#[cfg(not(unix))]
#[path = "baremetal.rs"]
mod imp;


pub use self::imp::find_cfi_sections;
