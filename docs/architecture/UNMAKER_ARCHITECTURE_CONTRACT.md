# UnMaker architecture contract

Status as of November 18, 2026: the owner accepted R0 earlier, and accepted R2 and R2.3 on this date. R3 is authorized. The tables below are the R0 record at `78ae99b`. Later code writes the Goomba `SpriteDef` and `AnimSet` on approve. Do not read a row that says "Approve does not create them" as the current Goomba path. The issue queue in the R0 record is still the R3 job. `issues` in `nesticles-core` holds the first decision rules and does not yet write the project file.

R0 proposal text follows. Grounded in `Project`, `SpriteDef`, `Candidate`, and `ContentPackage` as they exist at `78ae99b`.

## Lifecycle

`Source → Evidence → Candidate → Reconstruction → Issue → Verified asset → Creation → Package → Library → Maker`

| Stage | Record | Exists today |
| --- | --- | --- |
| Source | ROM bytes in process memory, plus payload SHA-256 | Yes, during the session. Bytes are not written to the project. |
| Evidence | `RawTile` index, bank, file offset. Documented `TileSpot`s. Reference `RefImage` | Indexes yes. No persisted link from a spot to a decoded picture. |
| Candidate | `Candidate` with `FrameDraft`s, status, attempts | Yes, in `ExtractionSession`, RAM only. |
| Reconstruction | One RGBA or part list per frame, still tied to tile indexes | Partial: `embed_form` draws the first frame only. |
| Issue | A specific unresolved question with a suggestion | No. `ReviewStatus` and `CoverageRow` are labels. |
| Verified asset | `SpriteDef` + `AnimSet` + provenance | The types exist. Approve does not create them. |
| Creation | User-edited frames, clips, anchors | `anim.rs` and `anim_ui.rs` can edit a sprite that is already in the project. |
| Package | `.nasset` JSON (`ContentPackage`) and `.nknowledge` (`Recipe`) | Yes. Package pictures are baked RGBA, not tile indexes. |
| Library | `AlphaLibrary` under the OS data dir | Recipes reload on the next extract. They do not reopen as an animation by themselves. |
| Maker | Place a blueprint whose picture is the sprite | Maker already places assets. It does not receive a Goomba reconstruction today. |

## What a character must retain

Identity and payload SHA-256: `Candidate` and `Recipe` have them. Tile indexes: `TileSpot.tile`. Arrangement: `TileSpot` x/y/flip. Palette: a `u8` slot and an unverified label. The baked picture uses a fixed gray ramp in `embed_form`. Frames and clip names: `FrameDraft.state`. Timing: `PackState.ticks` is hardcoded to 8 on export; `Clip.ticks` can store real values later. Origins and anchors: `SpriteDef` only, not `Candidate`. User corrections: `Attempt` and `rejected_tiles`. Unresolved work: coverage strings. Dependencies: not modeled. Blueprint: separate id, optional later. Style and category: strings on the candidate. Self-contained export: RGBA inside the package, so the ROM is not required after a successful package. That package is not a frame-accurate animation yet.

## Approve

Today: `approve_candidate` returns an error if there are no tile ids, otherwise sets `Approved`.

Required after R2: the same checks, then build one `SpriteFrame` per `FrameDraft` from CHR, one `Clip` per state, store provenance and the payload hash on the asset notes or a new serde-default field, mark the candidate approved, and mark the project dirty. Reopen loads that sprite from the project JSON. The ROM is not read. If the pictures were never baked, reopen shows the issue "pictures missing" and does not invent pixels.

## Relationships

| Link | How it is known |
| --- | --- |
| ROM offset to tile index | Deterministic. 16 bytes per NES tile from `chr_start`. |
| Tile index to sprite part | Deterministic only when a documented table or a verified recipe names the index and the x/y. Visual similarity is a hypothesis. |
| Parts to frame | Deterministic once spots exist. `place` already maps the six-byte OAM groups and drops `0xFC`. |
| Frames to clip | The documented state name (`walk`, `defeat`) becomes the clip name. |
| Clips to form | SMB1 enemies in the table are one form each. Power-up forms are out of the Goomba slice. |
| Form to character | `character_id`, for example `goomba`. |
| Character to blueprint | Optional and later. Patrol and jump labels are not physics. |

Confidence is `documented` for a hash-matched table, `recipe` for a local verified recipe, `user` after a correction, and `unverified` for palette and timing. A rejected tile list is an alternative that must not be suggested again unchanged. `reinspect` already refuses to present the same grouping as new.

AI may rank a reference crop against a rendered frame after the user consents. It may not invent tile indexes outside the CHR count (`validate_ai_proposal`). It may not set original timing. Screenshots do not prove NES frame durations or physics.

## Investigation

A bounded job, resumable by stage number stored with the reconstruction:

1. Hash header and payload (`inspect_nes`).
2. Raw tiles (`raw_tiles`). Stop if CHR length is zero.
3. Profile match on payload SHA-256 only (`smb1_profile`). No match means raw tiles and no names.
4. Apply verified recipes for that hash.
5. Expand documented spots into frame pictures.
6. Attach palette as unverified unless a recipe stores one.
7. Group states into clips. Default ticks stay labeled unverified.
8. Optional reference compare, R4.
9. Validate: every tile index in range, `0xFC` absent, at least one opaque pixel.
10. Emit issues for anything still unresolved. Mario and Luigi are issues, not silent success.

Cancellation drops the in-progress job and leaves saved files. Two identical failures with no new input stop that stage. Limits: existing ROM cap 64 MB, page 1 MB, image 4 MB, package and knowledge size checks. No network except the reference fetch the user starts. No AI without a key and consent.

## Issue queue

One issue at a time. Fields: asset id, question, evidence ids, attempts, one suggested change, alternatives, and the preview. Actions: accept, try alternative, investigate further, edit manually, skip, ignore, return later. Accept writes the sprite and a recipe. Reject appends an `Attempt` and excludes that tile tuple. Local learning is the recipe file on this machine. It is not uploaded.

## UI contract

Not built in R0. Target at 1366×768: white surfaces, light gray rules, dark type, small red accents, no Nintendo artwork. Screens: home (open ROM or project), investigation progress, gallery of reconstructions with a large preview, one issue with the preview updating on accept, animation editor, authoring, export. Tile indexes and URLs sit behind a details disclosure. The current parchment UnMaker strip stays until R1 replaces the flow. Clipped strip buttons at this width are a known defect, not the R0 task.

## Module boundaries for R1–R2

| Module | Owns |
| --- | --- |
| `pipeline` | Hash, raw tiles, profile, review status, recipes. Shrink frame baking into a tested function. |
| `anim` | Sprite and clip mutations. No second type. |
| New `reconstruct` in `nesticles-core` | CHR spots to per-frame RGBA, issue list, approve-into-project. Pure and deterministic. |
| UnMaker UI module | Progress, gallery, one issue. Does not own physics. |
| `app.rs` | Routes workspace. Does not grow a new extraction system. |
| Maker modules | Unchanged until the integration gate. |
