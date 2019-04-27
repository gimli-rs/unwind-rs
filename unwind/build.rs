extern crate cc;
use std::env;

fn main() {
    match env::var("CARGO_FEATURE_ASM") {
        Err(env::VarError::NotPresent) => {
            cc::Build::new()
                       .file("src/unwind_helper.c")
                       .compile("unwind_helper");
        },
        _ => ()
    }
}
