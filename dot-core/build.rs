// ABOUTME: Build script that compiles vendored Graphviz and generates Rust FFI bindings.
// ABOUTME: Produces static libraries and C bindings via cmake and bindgen.

use std::env;
use std::path::PathBuf;

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "wasm32" {
        // Parser-only build — no C dependencies needed
        return;
    }

    // Homebrew bison and flex are keg-only on macOS, so we must
    // point CMake at them explicitly.
    let homebrew_prefix = std::process::Command::new("brew")
        .arg("--prefix")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "/opt/homebrew".to_string());

    let bison_exe = format!("{}/opt/bison/bin/bison", homebrew_prefix);
    let flex_exe = format!("{}/opt/flex/bin/flex", homebrew_prefix);

    let dst = cmake::Config::new("graphviz-vendor")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("ENABLE_LTDL", "OFF")
        .define("WITH_GVEDIT", "OFF")
        .define("WITH_SMYRNA", "OFF")
        .define("WITH_EXPAT", "ON")
        .define("WITH_ZLIB", "ON")
        .define("GRAPHVIZ_CLI", "OFF")
        .define("ENABLE_TCL", "OFF")
        .define("ENABLE_SWIG", "OFF")
        .define("ENABLE_SHARP", "OFF")
        .define("ENABLE_D", "OFF")
        .define("ENABLE_GO", "OFF")
        .define("ENABLE_GUILE", "OFF")
        .define("ENABLE_JAVA", "OFF")
        .define("ENABLE_JAVASCRIPT", "OFF")
        .define("ENABLE_LUA", "OFF")
        .define("ENABLE_PERL", "OFF")
        .define("ENABLE_PHP", "OFF")
        .define("ENABLE_PYTHON", "OFF")
        .define("ENABLE_R", "OFF")
        .define("with_cxx_api", "OFF")
        .define("with_cxx_tests", "OFF")
        .define("BISON_EXECUTABLE", &bison_exe)
        .define("FLEX_EXECUTABLE", &flex_exe)
        .build();

    let lib_dir = dst.join("lib");
    let build_dir = dst.join("build");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Plugins are built but not installed by cmake install, so we need
    // to add search paths for each plugin and internal library directory.
    // These paths match the Graphviz 12.2.1 build structure.
    let search_subdirs = &[
        "plugin/core",
        "plugin/dot_layout",
        "plugin/neato_layout",
        "lib/common",
        "lib/dotgen",
        "lib/neatogen",
        "lib/fdpgen",
        "lib/circogen",
        "lib/twopigen",
        "lib/osage",
        "lib/patchwork",
        "lib/sfdpgen",
        "lib/sparse",
        "lib/pack",
        "lib/ortho",
        "lib/label",
        "lib/rbtree",
        "lib/vpsc",
        "lib/util",
        "lib/ast",
        "lib/sfio",
        "lib/vmalloc",
        "lib/expr",
        "lib/gvpr",
        "lib/edgepaint",
    ];
    for subdir in search_subdirs {
        let path = build_dir.join(subdir);
        if path.exists() {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
    }

    // Link Graphviz plugin static libraries
    for lib in &[
        "gvplugin_dot_layout",
        "gvplugin_neato_layout",
        "gvplugin_core",
    ] {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    // Link Graphviz core static libraries (installed to lib/)
    for lib in &["gvc", "cgraph", "cdt", "pathplan", "xdot"] {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    // Link internal Graphviz libraries needed by plugins and core
    for lib in &[
        "common",
        "dotgen",
        "neatogen",
        "fdpgen",
        "circogen",
        "twopigen",
        "osage",
        "patchwork",
        "sfdpgen",
        "sparse",
        "pack",
        "ortho",
        "label",
        "rbtree",
        "vpsc",
        "util",
    ] {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    // System libraries
    println!("cargo:rustc-link-lib=expat");
    println!("cargo:rustc-link-lib=z");

    // GTS/glib — neatogen uses GTS for Delaunay triangulation if available at
    // cmake configure time. Link the dynamic libraries so symbols resolve.
    println!("cargo:rustc-link-lib=gts");
    println!("cargo:rustc-link-lib=glib-2.0");
    println!("cargo:rustc-link-lib=gthread-2.0");
    println!("cargo:rustc-link-lib=gmodule-2.0");
    println!("cargo:rustc-link-lib=intl");
    println!("cargo:rustc-link-search=native={}/opt/gts/lib", homebrew_prefix);
    println!("cargo:rustc-link-search=native={}/opt/glib/lib", homebrew_prefix);
    println!("cargo:rustc-link-search=native={}/opt/gettext/lib", homebrew_prefix);
    println!("cargo:rustc-link-search=native={}/lib", homebrew_prefix);

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
    #[cfg(not(target_os = "macos"))]
    println!("cargo:rustc-link-lib=stdc++");

    // Generate Rust bindings from the C headers
    let include_dir = dst.join("include");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include_dir.display()))
        .allowlist_function("gvContext")
        .allowlist_function("gvContextPlugins")
        .allowlist_function("gvAddLibrary")
        .allowlist_function("gvLayout")
        .allowlist_function("gvFreeLayout")
        .allowlist_function("gvRenderData")
        .allowlist_function("gvFreeRenderData")
        .allowlist_function("gvFreeContext")
        .allowlist_function("agmemread")
        .allowlist_function("agclose")
        .allowlist_function("agerrors")
        .allowlist_function("aglasterr")
        .opaque_type("FILE")
        .generate()
        .expect("Unable to generate Graphviz bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("graphviz_bindings.rs"))
        .expect("Couldn't write bindings");

    // Re-run if graphviz source or wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=graphviz-vendor/");
}
