# Known limitations — Phase 9.1 test candidate

This package is not a final accepted Phase 9.1 build.

- The authentic Super Mario Bros. acceptance pass needs your cartridge dump on your machine. A synthetic fixture and a hash check are what this build was tested with.
- Mario and Luigi player tiles are not in the shipped profile. Those coverage rows stay unresolved. Enemy rows that match the public disassembly are proposals until you approve them.
- Palettes are not stored in CHR. Palette status stays unverified.
- Live OpenAI vision and image generation need your key in the app. This build was tested with an empty key, which reports invalid credentials, and with mocked HTTP bodies in unit tests. That is not a live acceptance pass.
- Patrol and jump on a derived behavior are labels copied from the parent graph. Rust Play steps the returning projectile. It does not replace Hammer Bro's existing SMB1 movement with a second physics model.
- Godot preview does not play Phase 9 animation clips. That remains later work.
- The next visual pass is a white clinical layout. These screens keep the current theme.
- `cargo fmt --all --check` still fails on eight older files: `audio.rs`, `codec.rs`, `detect.rs`, `wav.rs`, `audio_out.rs`, `icon.rs`, `keys.rs`, and `net.rs`. They were not reformatted.
- This cloud desktop is not Bazzite or KDE.
