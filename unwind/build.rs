extern crate gcc;
use std::env;

fn main() {
    match env::var("CARGO_FEATURE_NIGHTLY") {
        Err(env::VarError::NotPresent) => {
            gcc::Build::new()
                       .file("src/unwind_helper.c")
                       .compile("unwind_helper");
        },
        _ => ()
    }
}
