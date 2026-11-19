# Nesticles R2.3 — Direct Visual Polish Source Patch

This is a visual-only source modification prepared from the owner's uploaded `Nesticles-Source.zip`. It is **not** a compiled release and has **not** been run in Fedora/Bazzite. The sandbox used to prepare it did not have `cargo` or `rustc` installed.

## Changes implemented in source

- Replaced the native application PNG with an original transparent pixel-art yellow egg icon, for the KDE taskbar and window.
- Replaced minimal geometric marks with more detailed original **yellow speckled egg** and **castle** vector painters. These are interface marks, not cartridge graphics.
- Added shared custom **glossy/beveled surface painter** with highlights and borders; strengthened soft shadows and common control corner radii.
- Upgraded the UnMaker header/workspace switches, blue selected navigation states, visual hierarchy, card depth, and Discover inspector appearance.
- Added a muted, decorative sky/cloud/hill header to Discover and a quiet scenic accent at the bottom of UnMaker navigation.
- Increased Review playback/state control hit targets and differentiated the preview heading from raw evidence text.
- Gave Maker floating parts/tool panels more readable headings, tactile highlighted tool buttons with icons, and a decorative landscape for empty/default-sky canvas presentation. The scenery **does not create course cells, collision, levels, export content or ROM-derived sprites**. Nondefault course skies are not replaced.
- Preserved the existing Discover/gallery layout and the Maker tools' placement/behavior event handlers as requested.

## Modified files

- `crates/nesticles/assets/icon.png`
- `crates/nesticles/src/icon.rs`
- `crates/nesticles/src/design.rs`
- `crates/nesticles/src/studio.rs`
- `crates/nesticles/src/unmaker/view.rs`
- `crates/nesticles/src/app.rs`

## Known limitations and follow-up checks

- Rust build is **unverified** here: this environment has no Rust compiler or Cargo and cannot fetch toolchains. The owner should compile and run on their existing configured development environment.
- The currently available content is unchanged. The app does not magically gain Mario/Koopa artwork, new game styles or an asset library.
- Background decorations are programmatic vector painters, not copyright-controlled spritesheets.
- This is a **styling pass**; it does not claim to fix missing mouse click-path acceptance, unresolved R2 storage issues or incomplete future tools.
- Verify egui painting performance and viewport behavior on the actual Bazzite desktop, including a maximized and a restored window.
- Review the new transparent/decorative surfaces against actual user assets for readability and contrast.

## Local build and run

From a Rust-enabled Fedora development environment:

```bash
cd NES_Plus
cargo test --offline -p nesticles
cargo build --release --locked -p nesticles
./target/release/nesticles
```

If offline compilation reports an unavailable crate, use the existing project's dependency workflow rather than removing security or version constraints. Do not install the test application globally.
