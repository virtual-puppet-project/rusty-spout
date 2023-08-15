use std::path::{Path, PathBuf};

/// The Spout2 fork that does not include precompiled `dll`s and `lib`s.
const SPOUT_DIR: &str = "Spout2-lean";
const SPOUT_TAG: &str = "2.007.011";

fn main() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    ensure_spout_initted();
    let (spout_build_dir, lib_dir) = build_spout();

    if let Err(e) = std::fs::write(
        repo_root.join("_spout_dll_path"),
        spout_build_dir
            .join("bin/SpoutLibrary.dll")
            .to_str()
            .unwrap(),
    ) {
        println!("cargo:warning={e}");
    }

    let mut cxx_builder = autocxx_build::Builder::new(
        "src/lib.rs",
        &[spout_build_dir.join("include/SpoutLibrary")],
    )
    .build()
    .unwrap();
    cxx_builder
        .flag_if_supported("-std=c++14")
        .compile("spoutlib");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=lib.rs");

    println!("cargo:rustc-link-lib=SpoutLibrary");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
}

fn ensure_spout_initted() {
    if !Path::new(SPOUT_DIR).exists() {
        let status = std::process::Command::new("git")
            .args(["submodule", "update", "--init", SPOUT_TAG])
            .status()
            .unwrap();

        if !status.success() {
            panic!("Unable to init Spout2 submodule");
        }
    }
}

fn build_spout() -> (PathBuf, PathBuf) {
    let dst = cmake::Config::new(SPOUT_DIR)
        .define("SKIP_INSTALL_ALL", "OFF")
        .define("SKIP_INSTALL_HEADERS", "OFF")
        .define("SKIP_INSTALL_LIBRARIES", "OFF")
        .define("SPOUT_BUILD_CMT", "OFF")
        // The only one we want
        .define("SPOUT_BUILD_LIBRARY", "ON")
        .define("SPOUT_BUILD_SPOUTDX", "OFF")
        .define("SPOUT_BUILD_SPOUTDX_EXAMPLES", "OFF")
        .build();

    (dst.clone(), dst.join("lib"))
}
