use cbindgen;
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
      .with_crate(&crate_dir)
      .with_config(cbindgen::Config::from_file(crate_dir + "/cbindgen.toml").unwrap())
      .generate()
      .expect("Unable to generate bindings")
      .write_to_file("include/nogasm.h");
}