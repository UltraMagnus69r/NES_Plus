# Goomba vertical slice

R2 implements the walk and defeat described below for the verified SMB1 payload. The R0 plan text stays as the record of that plan.

The walk is not four separate tiles. `place` builds one 16×24 metasprite from `70 71 72 73`, leaving the top row empty because those slots are `FC`. The second walk pose is a horizontal mirror of that picture. This payload contains `A5 09 29 08 D0 06 A5 03 49 03`, which is the graphics handler test that mirrors the sprite for 8 frames. Defeat is `EF` on the left and `EF` flipped on the right, and the clip plays once.

The baked colors are the review palette. CHR does not contain the PPU palette, so those colors are labeled "Review colors only — original palette not yet verified." The R0 note that said to use a gray ramp is superseded by that labeled review palette. Approval writes the sprite into the project. It does not store the ROM. Discover shows a compact card. Review is the one place that plays Walk and Defeat / Stomped and offers Approve & Save. Those two states belong to one default form. A future shell or a Mario power-up form would be another state or form on that same sheet once the graphics are actually verified. They are not generated now. R2 is not accepted.

R0 plan:

## Proof

Owner ROM `/home/g/Desktop/Roms/smb1.nes`, headerless payload `fcb6a0ef3a20c19b356005fbb21dc8009563b1cb5a9aaebc8e9386b4a8c5912e`, becomes an animated Goomba in the project file. Save, close, reopen. The animation still plays. The ROM is not read on reopen. No API key.

If that file or hash is absent, the slice is BLOCKED. The synthetic fixture is not a substitute.

## What the code already does

`smb1_documented` in `crates/nesticles-core/src/pipeline.rs` builds a Goomba candidate when the payload hash matches. Walk spots are tiles `0x70`, `0x71`, `0x72`, `0x73`. Defeat spots are tile `0xEF` twice. `place` drops `0xFC`. `embed_form` can decode those tiles with `codec::decode_tile` and a gray ramp, but only the first frame is blitted, and `import_package` attaches every clip to one sprite frame.

## Smallest R2 change

1. Add `reconstruct_frames` next to `embed_form`. Input: ROM bytes, one `Candidate`. Output: one RGBA buffer per `FrameDraft`, width and height from the spots. Use the existing gray ramp and mark palette unverified. Do not invent an SMB1 palette.
2. Set clip ticks to 8 and mark timing unverified. Do not claim original NES durations.
3. On approve, call `anim::create_sprite` and add one frame per buffer, then `create_set` and one clip per state (`walk` loop, `defeat` once). Parts point at assets that hold those RGBA buffers. Tile indexes stay in the recipe.
4. Save the project. Reopen uses the project JSON only.
5. Rust Play or the existing anim preview samples the clips. Godot is out of this slice.

Files: `pipeline.rs` or a new `reconstruct.rs`, `anim.rs` only if a helper is missing, `pipeline_ui.rs` or the new UnMaker module for the approve button, tests in the core crate. No `app.rs` growth beyond one call.

## Tests

Unit: a tiny CHR fixture with four known tiles and `0xFC` produces two different RGBA frames and does not name them Goomba. Hash mismatch produces no Goomba name.

Integration: approve then `Project` JSON reload still has two frames and two clips. Package JSON contains no ROM bytes and no `sk-`.

Acceptance, owner machine only: open `smb1.nes`, confirm the payload hash, see two distinct previews, approve, quit, reopen, preview still moves. Record that palette and timing are unverified.

## Out of scope

Mario, Luigi, other enemies, reference-page matching, AI, Blueprint physics, the white UI theme, Phase 10.
