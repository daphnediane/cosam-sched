// build.rs for cosam-convert
//
// Runs `npm run build` to produce the minified widget JS/CSS before the Rust
// compiler tries to include_str! them.  This means `cargo build` is the only
// command a developer needs; Node.js >= 18 must be installed.

use std::path::PathBuf;

fn main() {
    // Locate repo root (two levels up from apps/cosam-convert/).
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = PathBuf::from(&manifest_dir).join("../..").canonicalize()
        .expect("Could not resolve repo root from CARGO_MANIFEST_DIR");

    // Tell Cargo to re-run this script only when the widget sources change.
    for path in [
        "widget/cosam-calendar.js",
        "widget/cosam-calendar.css",
        "widget/build.mjs",
        "package.json",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }

    // Ensure npm dependencies are present.
    let node_modules = root.join("node_modules");
    if !node_modules.exists() {
        eprintln!("cargo:warning=node_modules not found; running npm install...");
        let status = std::process::Command::new("npm")
            .arg("install")
            .current_dir(&root)
            .status()
            .expect(
                "Failed to run `npm install`. Is Node.js >= 18 installed and on PATH?",
            );
        if !status.success() {
            panic!("npm install failed — cannot build widget assets.");
        }
    }

    // Build the widget.
    let status = std::process::Command::new("npm")
        .args(["run", "build"])
        .current_dir(&root)
        .status()
        .expect("Failed to run `npm run build`. Is Node.js >= 18 installed and on PATH?");

    if !status.success() {
        panic!("npm run build failed — cannot build widget assets.");
    }
}
