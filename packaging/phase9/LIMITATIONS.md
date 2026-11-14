# Nesticles Alpha 2 — Phase 9 limitations

- Rust Play drives idle, walk, jump, fall, and hurt for the player, and walk and defeat for an enemy, from the simulation. Run, skid, swim, climb, fly, and the other named states can be authored and previewed. They do not change playback in Play yet.
- Godot 4.7.2 preview still runs the earlier subset: walk, run, jump, solids, goombas, and goals. It does not play Phase 9 animation clips. That playback is Phase 19.
- A sprite edge is at most 256 pixels. A sprite holds at most 48 frames and 24 pieces. An animation holds at most 24 states. A project holds at most 64 sprites and 64 animations. Undo keeps 32 sprite edits.
- Baked PNG and RGBA colors are kept. Nesticles does not invent NES palette indices from those colors.
- The original cartridge is not stored in the project. Import the pictures you need before you move the cartridge.
- A picture never becomes a Blueprint by itself. Create or select a Blueprint, then assign the animation.
- Cloud and XFCE checks do not establish Bazzite or KDE behavior.
- Optional workshop assistant and image maker still need their own keys. Sprite assembly and animation do not.
