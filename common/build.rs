extern crate cbindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_file = PathBuf::from(&crate_dir)
        .join("..")
        .join("components")
        .join("hcp_bridge")
        .join("shared_data.h");

    // Ensure the output directory exists
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    cbindgen::generate(crate_dir)
        .expect("Unable to generate bindings")
        .write_to_file(output_file);
}
