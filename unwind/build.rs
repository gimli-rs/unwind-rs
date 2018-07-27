extern crate gcc;
use std::env;

fn main() {
    match env::var("CARGO_FEATURE_NIGHTLY") {
        Err(env::VarError::NotPresent) => (),
        _ => return
    }

    gcc::Build::new()
        .file(format!("src/glue/{}_helper.S", env::var("CARGO_CFG_TARGET_ARCH").expect("Didn't run with cargo")))
        .include("src/glue")
        .compile("unwind_helper");
}
