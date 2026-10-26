// Compiles bgsage's CPU engine (the `bgbot_lib` sources of its
// cpp/CMakeLists_cpu.txt) and its C interface, cpp/src/mobile.cpp, without
// CMake or Python.

use std::path::PathBuf;

const SOURCES: &[&str] = &[
    "board", "moves", "strategy", "pubeval", "game", "benchmark", "encoding",
    "neural_net", "training", "multipy", "rollout", "cube", "cube_eval",
    "match_equity", "bearoff", "mobile",
];

fn main() {
    let cpp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vendor/bgsage/cpp");
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(false).include(cpp.join("include")).opt_level(3).flag_if_supported("-funroll-loops");
    // bgsage's neural_net.cpp always takes its AVX2 path on x86, so these are
    // required there, not optional: the engine needs an AVX2 CPU (2013 on).
    match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => {
            build.flag("-mavx2").flag("-mfma").flag("-ffast-math");
        }
        Ok("aarch64") => {
            build.flag("-funsafe-math-optimizations").flag("-fno-math-errno");
        }
        _ => {}
    }
    for name in SOURCES {
        let file = cpp.join("src").join(format!("{name}.cpp"));
        println!("cargo:rerun-if-changed={}", file.display());
        build.file(file);
    }
    println!("cargo:rerun-if-changed={}", cpp.join("include").display());
    build.compile("bgsage");
    println!("cargo:rustc-link-lib=pthread");
}
