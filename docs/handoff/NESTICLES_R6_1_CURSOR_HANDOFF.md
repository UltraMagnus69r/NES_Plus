# Nesticles — R6.1 visual handoff to Cursor

## Workspace

- Open the **NES_Plus** directory in Cursor (the folder containing Cargo.toml).
- R6.1 is the **visual iteration label**, not approval to advance historical phase gates.
- The user confirmed the R6.1 app launches successfully on Bazzite. No claim is made that all editor interactions, resizing, exports, save/load, or acceptance scenarios have been validated.
- Keep AGENTS.md and .cursor/rules authoritative; review the roadmap and gate policies before new functionality.

## UI state and assets

- Native Rust + egui/eframe desktop app; two workspaces: UnMaker Lab and Maker Engine.
- Glossy rounded buttons, default/selected icons, Discover hero, sidebars and Maker wallpaper are integrated into crates/nesticles/src and crates/nesticles/assets/ui.
- Optional local artwork overrides: ~/Desktop/Nesticle/Assets/ ; do not overwrite user artwork.
- User cartridge ROMs: ~/Desktop/Nesticle/ROMS/ ; do not move, commit, or redistribute ROM files.
- The Maker course is an editable blank tile grid; the colorful landscape is decorative editor wallpaper, not a game level.
- R6 added drag-to-resize with safeguards; verify resizing behavior and save/reopen persistence before calling it accepted.

## Build and run on Bazzite

Open Konsole in this directory and run:

```bash
bash run-r6.sh
```

The launcher uses Distrobox `nesticles-visual-dev` if available. For a previously built executable:

```bash
bash run-r6.sh --run-only
```

Build errors: ~/Desktop/nesticles-r6-build.log
Runtime errors: ~/Desktop/nesticles-r6-runtime.log

## Mandatory next checks

1. Open both workspaces. Confirm the section header banners and Tool sidebar art do not distort/cover controls.
2. Import the user's cartridge from its external ROMS location and inspect Discover / Review.
3. Create an empty course, verify grid units, resize wider/taller and smaller safely.
4. Save, close, reopen and confirm project state and course dimensions persist.
5. Repeat the authoring and replay flow with user-supplied cartridge assets as required by the active roadmap.
6. Run tests and capture exact results. Never treat unverified claims as passing tests.

## Git policy

The migration did not alter .git, checkout branches, stage, commit, or push changes.
Review `git status` in Cursor before deciding whether to commit the R6.1 baseline.
