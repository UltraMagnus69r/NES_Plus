# Nesticles R3 – Integrated UI asset + button pass

## Scope

This is a direct modification of the native Rust/egui application, based on the last supplied corrected source (not a UI mockup, prompt, web app, or refactor of gameplay systems).

### Changes

- Added `src/ui_assets.rs`: a texture loader that caches 59 images on startup. It reads optional replacement files from `$NESTICLES_ASSETS_DIR` or `$HOME/Desktop/Nesticle/Assets`, falling back to embedded images. All textures in the ZIP are embedded; the user's assets folder is not required to launch.
- Added 52 individualized default/selected navigation, tools, and gallery-filter icons, plus egg logo, wordmark, magnifier and backgrounds.
- Added a shared manually rendered `capsule_button` in `design.rs`: 5 semantic colors, bright gloss, rounded radius, shadow, hover/pressed states, and centered icon+text alignment.
- Updated global egui widget rounding, colors, and text/padding for buttons that use the standard egui widget API.
- Updated the header with the actual Nesticles artwork, glossy workspace selectors and action buttons.
- UnMaker rail draws the supplied light-blue and green-hill full-height backdrop; stage buttons use the appropriate nav icon and selected image.
- Discover draws the supplied hillside hero banner, actual magnifying glass, all 11 individually iconized filter categories, and a stronger inspector CTA.
- Discover gallery cells use consistent card dimensions to correct the stepped layout. Click a card to select its inspector; double-click to enter Review.
- Maker uses the 16:9 supplied platformer wallpaper on the editor (presentation only, never written as course cells), with iconized category chips, tool controls and styled play buttons.
- Maker game-style selector now uses a compact pill and only exposes existing style support; future styles remain disabled.
- Native play/build/extraction logic otherwise retained from the supplied source.

## Verification disclosure

This package was statically inspected and its image files validated. It was **not compiled or GUI-tested** in the assistant runtime because Rust/Cargo and network package installation were unavailable there. It is a **build-from-source test release**, not an experimentally verified native executable. Build results on Bazzite must be checked before deployment.

The earlier SMB1 test includes a machine-specific `/home/g/Desktop/Roms/smb1.nes` path. The launcher performs `cargo build --release --locked -p nesticles` but does not run that environment-specific test.

## Runtime asset folder

The expected override path is `/home/UltraMagnus69r/Desktop/Nesticle/Assets/`, with a nested file layout matching `crates/nesticles/assets/ui/`. `run-r3.sh` copies bundled UI files there **without overwriting any files already at that destination**. To use a custom asset, replace its corresponding path and relaunch the program; images are loaded at startup.

Example: `Assets/icons/navigation/selected/discover.png`.

The supplied backdrop textures are UI decoration only. They do not create fictional ROM sprites, imported gameplay logic, or serialized project cells.
