use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let pkg = manifest.join("../nesticles-player/pkg");
    let wasm = pkg.join("nesticles_player_bg.wasm");
    let js = pkg.join("nesticles_player.js");
    let failure_path = pkg.join("BUILD_FAILURE.txt");
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("player_embed.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if wasm.is_file() && js.is_file() {
        let wasm_p = wasm.canonicalize().unwrap_or(wasm);
        let js_p = js.canonicalize().unwrap_or(js);
        std::fs::write(
            &dest,
            format!(
                "pub const READY: bool = true;\n\
                 pub const FAILURE: &str = \"\";\n\
                 pub static JS: &str = include_str!({js:?});\n\
                 pub static WASM: &[u8] = include_bytes!({wasm:?});\n",
                js = js_p,
                wasm = wasm_p
            ),
        )
        .unwrap();
        println!("cargo:rerun-if-changed={}", wasm_p.display());
        println!("cargo:rerun-if-changed={}", js_p.display());
    } else {
        let failure = std::fs::read_to_string(&failure_path).unwrap_or_else(|_| {
            "wasm32 player was not built yet. Run scripts/build-player.sh, then rebuild the desktop app so the export can embed it.".into()
        });
        std::fs::write(
            &dest,
            format!(
                "pub const READY: bool = false;\n\
                 pub const FAILURE: &str = {failure:?};\n\
                 pub static JS: &str = \"\";\n\
                 pub static WASM: &[u8] = &[];\n"
            ),
        )
        .unwrap();
        println!("cargo:rerun-if-changed={}", failure_path.display());
    }
}
