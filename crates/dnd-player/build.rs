//! Packs the standard content (`content/` at the workspace root) into the
//! player app, so a character can be built before the app has ever reached a
//! DM's server. Once it connects, the campaign's own list replaces this one.
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let content = manifest.join("../../content");
    println!("cargo:rerun-if-changed={}", content.display());
    println!("cargo:rerun-if-changed=build.rs");

    let mut lib = dnd_core::Library::new();
    if let Err(e) = lib.load_pack(&content) {
        panic!("could not load the content pack at {}: {e:#}", content.display());
    }
    let options = dnd_core::builder::builder_options(&lib, 1);
    assert!(!options.species.is_empty() && !options.classes.is_empty(), "content pack at {} has no species or classes", content.display());
    let json = serde_json::to_vec(&options).expect("builder options to JSON");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("builder.json");
    std::fs::write(&out, json).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
}
