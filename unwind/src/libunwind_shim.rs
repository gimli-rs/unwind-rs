#![allow(non_camel_case_types, non_snake_case, unused_variables)]

use libc::{c_void, c_int};
use fallible_iterator::FallibleIterator;
use gimli::X86_64;

use registers::Registers;
use super::{DwarfUnwinder, Unwinder, StackFrames};

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub enum _Unwind_Action {
    _UA_SEARCH_PHASE = 1,
    _UA_CLEANUP_PHASE = 2,
    _UA_HANDLER_FRAME = 4,
    _UA_FORCE_UNWIND = 8,
    _UA_END_OF_STACK = 16,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum _Unwind_Reason_Code {
    _URC_NO_REASON = 0,
    _URC_FOREIGN_EXCEPTION_CAUGHT = 1,
    _URC_FATAL_PHASE2_ERROR = 2,
    _URC_FATAL_PHASE1_ERROR = 3,
    _URC_NORMAL_STOP = 4,
    _URC_END_OF_STACK = 5,
    _URC_HANDLER_FOUND = 6,
    _URC_INSTALL_CONTEXT = 7,
    _URC_CONTINUE_UNWIND = 8,
    _URC_FAILURE = 9, // used only by ARM EHABI
}
pub type _Unwind_Exception_Class = u64;
pub type _Unwind_Exception_Cleanup_Fn = extern "C" fn(unwind_code: _Unwind_Reason_Code,
                                                      exception: *mut _Unwind_Exception);
#[repr(C)]
pub struct _Unwind_Exception {
    pub exception_class: _Unwind_Exception_Class,
    pub exception_cleanup: _Unwind_Exception_Cleanup_Fn,
    pub private_contptr: Option<u64>,
    //pub private: [_Unwind_Word; 20],
}

pub type _Unwind_Word = usize;
pub type _Unwind_Ptr = usize;
pub struct _Unwind_Context {
    pub lsda: u64,
    pub ip: u64,
    pub initial_address: u64,
    pub registers: *mut Registers,
}
pub type _Unwind_Trace_Fn = extern "C" fn(ctx: *mut _Unwind_Context, arg: *mut c_void)
                                          -> _Unwind_Reason_Code;
type PersonalityRoutine = extern "C" fn(version: c_int, actions: c_int, class: u64, object: *mut _Unwind_Exception, context: *mut _Unwind_Context) -> _Unwind_Reason_Code;

// FIXME: we skip over this function when unwinding, so we should ensure
// it never needs any cleanup. Currently this is not true.
#[no_mangle]
pub unsafe extern "C" fn _Unwind_Resume(exception: *mut _Unwind_Exception) -> ! {
    ::glue::registers(|registers| unwind_tracer(registers, exception));
    unreachable!();
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_DeleteException(exception: *mut _Unwind_Exception) {
    ((*exception).exception_cleanup)(_Unwind_Reason_Code::_URC_FOREIGN_EXCEPTION_CAUGHT, exception);
    trace!("exception deleted.");
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_GetRegionStart(ctx: *mut _Unwind_Context) -> _Unwind_Ptr {
    (*ctx).initial_address as usize
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_GetTextRelBase(ctx: *mut _Unwind_Context) -> _Unwind_Ptr {
    unreachable!();
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_GetDataRelBase(ctx: *mut _Unwind_Context) -> _Unwind_Ptr {
    unreachable!();
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_GetLanguageSpecificData(ctx: *mut _Unwind_Context) -> *mut c_void {
    (*ctx).lsda as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_SetGR(ctx: *mut _Unwind_Context, reg_index: c_int, value: _Unwind_Word) {
    (*(*ctx).registers)[reg_index as u16] = Some(value as u64);
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_SetIP(ctx: *mut _Unwind_Context, value: _Unwind_Word) {
    (*(*ctx).registers)[X86_64::RA] = Some(value as u64);
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_GetIPInfo(ctx: *mut _Unwind_Context, ip_before_insn: *mut c_int)
                                    -> _Unwind_Word {
    *ip_before_insn = 0;
    (*ctx).ip as usize
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_FindEnclosingFunction(pc: *mut c_void) -> *mut c_void {
    pc // FIXME: implement this
}

// FIXME: Set `unwind(allowed)` because we need to be able to unwind this function as
// part of its operation. But this means any panics in this function are undefined
// behaviour, and we don't currently ensure it doesn't panic.
//
// On stable (1.32), `unwind(allowed)` is the default, but this will change in 1.33, with
// no stable way of setting `unwind(allowed)`, so this function will always abort in 1.33.
#[cfg_attr(feature = "nightly", unwind(allowed))]
#[no_mangle]
pub unsafe extern "C" fn _Unwind_RaiseException(exception: *mut _Unwind_Exception) -> _Unwind_Reason_Code {
    (*exception).private_contptr = None;
    ::glue::registers(|registers| unwind_tracer(registers, exception));
    unreachable!();
}

unsafe fn unwind_tracer(registers: Registers, exception: *mut _Unwind_Exception) {
    let mut unwinder = DwarfUnwinder::default();
    let mut frames = StackFrames::new(&mut unwinder, registers);

    if let Some(contptr) = (*exception).private_contptr {
        loop {
            if let Some(frame) = frames.next().unwrap() {
                if frames.registers()[X86_64::RSP].unwrap() == contptr {
                    break;
                }
            } else {
                return;
            }
        }
    }

    while let Some(frame) = frames.next().unwrap() {
        if let Some(personality) = frame.personality {
            trace!("HAS PERSONALITY");
            let personality: PersonalityRoutine = ::std::mem::transmute(personality);

            let mut ctx = _Unwind_Context {
                lsda: frame.lsda.unwrap(),
                ip: frames.registers()[X86_64::RA].unwrap(),
                initial_address: frame.initial_address,
                registers: frames.registers(),
            };

            (*exception).private_contptr = frames.registers()[X86_64::RSP];

            // ABI specifies that phase 1 is optional, so we just run phase 2 (CLEANUP_PHASE)
            match personality(1, _Unwind_Action::_UA_CLEANUP_PHASE as c_int, (*exception).exception_class,
                              exception, &mut ctx) {
                _Unwind_Reason_Code::_URC_CONTINUE_UNWIND => (),
                _Unwind_Reason_Code::_URC_INSTALL_CONTEXT => ::glue::land(frames.registers()),
                x => panic!("wtf reason code {:?}", x),
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_Backtrace(trace: _Unwind_Trace_Fn,
                                    trace_argument: *mut c_void)
                                           -> _Unwind_Reason_Code {
    DwarfUnwinder::default().trace(|frames| {
        while let Some(frame) = frames.next().unwrap() {
            let mut ctx = _Unwind_Context {
                lsda: frame.lsda.unwrap_or(0),
                ip: frames.registers()[X86_64::RA].unwrap(),
                initial_address: frame.initial_address,
                registers: frames.registers(),
            };

            trace(&mut ctx, trace_argument);
        }
    });
    _Unwind_Reason_Code::_URC_END_OF_STACK
}
