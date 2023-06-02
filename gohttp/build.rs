///
/// Created by gngpp <gngppz@gmail.com>
///
extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg("./build.sh")
        .output()
        .unwrap();
    if !out.status.success() {
        panic!(
            "Failed to build a static library, error: {}",
            String::from_utf8(out.stderr).unwrap()
        );
    }

    let workdir = env::current_dir().unwrap();
    let lib_path = PathBuf::from(workdir.join("ffi"));

    println!("cargo:rustc-link-search={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=gohttp");

    let bindings = bindgen::Builder::default()
        .header("ffi/libgohttp.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("unable to generate hello bindings");

    let out_path = PathBuf::from(workdir.join("src")).join("bindings.rs");
    bindings
        .write_to_file(&out_path)
        .expect("couldn't write bindings!");
    prepend_allow_warnings(out_path).unwrap()
}

fn prepend_allow_warnings(file_path: PathBuf) -> std::io::Result<()> {
    use std::fs::{read_to_string, write};
    let contents = read_to_string(&file_path)?;
    let new_contents = format!("#![allow(warnings)]\n{}", contents);
    write(file_path, new_contents)?;
    Ok(())
}
