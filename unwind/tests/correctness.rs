extern crate unwind;
extern crate backtrace;
extern crate fallible_iterator;

use unwind::{Unwinder, DwarfUnwinder};
use fallible_iterator::FallibleIterator;

#[test]
fn correctness() {
    test_frame_1();
}

#[inline(never)]
fn test_frame_1() { test_frame_2() }

#[inline(never)]
fn test_frame_2() { test_frame_3() }

#[inline(never)]
fn test_frame_3() {
    let bt = backtrace::Backtrace::new_unresolved();
    let ref_trace: Vec<u64> = bt.frames().iter().map(|x| x.ip() as u64).collect();

    for i in &ref_trace {
        println!("{:08x}", i);
    }
    println!();

    let mut ref_trace_len = ref_trace.len();
    // Haven't investigated why this has a trailing zero.
    if ref_trace[ref_trace_len - 1] == 0 {
        ref_trace_len -= 1;
    }

    let mut our_trace = Vec::new();

    DwarfUnwinder::default().trace(|frames| {
        // skip 3 (unwind-rs + test_frame_3)
        frames.next().unwrap();
        frames.next().unwrap();
        frames.next().unwrap();

        while let Some(_) = frames.next().unwrap() {
            our_trace.push(frames.registers()[16].unwrap() - 1);
        }
    });

    for i in &our_trace {
        println!("{:08x}", i);
    }

    let our_trace_len = our_trace.len();
    assert!(our_trace_len > 3);
    let ref_trace = &ref_trace[ref_trace_len - our_trace_len..][..our_trace_len];
    assert_eq!(our_trace, ref_trace);
}
