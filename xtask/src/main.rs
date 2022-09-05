//! This executable defines the `cargo xtask` subcommands.
//!
//! At the moment it is very simple, but if this grows more subcommands then
//! it will be sensible to use `clap` or another similar library.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub fn main() {
    let arg = env::args().nth(1);
    match arg.as_deref() {
        Some("codegen") => codegen(),
        _ => {
            eprintln!("unknown xtask");
            std::process::exit(-1);
        }
    }
}

/// `cargo xtask codegen`
///
/// This generates the header files for test libraries.
fn codegen() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = manifest_dir.parent().unwrap();

    // ffizz-tests-simplib header
    let simplib_crate_dir = workspace_dir.join("tests").join("simplib");
    let mut file = File::create(simplib_crate_dir.join("simplib.h")).unwrap();
    write!(&mut file, "{}", ffizz_tests_simplib::generate_header()).unwrap();
}
