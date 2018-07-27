#[cfg(target_arch = "x86_64")]
#[path = "x86_64.rs"]
mod imp;

#[cfg(target_arch = "aarch64")]
#[path = "aarch64.rs"]
mod imp;

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
compiler_error!("Unsupported architecture");

pub use self::imp::{unwind_trampoline, land};
// TODO: doc hidden
pub use self::imp::unwind_recorder;
