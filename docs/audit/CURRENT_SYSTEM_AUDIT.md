# Current system audit

Inspected at `78ae99be642075563f55f7dbe5960982f7c2a971` on `cursor/phase91-reference-url`. No gameplay code was changed for this audit.

Status as of November 18, 2026: this audit is historical. R2 later writes the Goomba sprite on approve. The accepted presentation is the R6.1 chrome, which is a visual label, not roadmap R6. Rows that describe the pre-R2 approve path or the parchment UnMaker strip describe that inspection, not the current studio.

A passing unit test is not a KEEP by itself. KEEP means the code can serve the product workflow without a new model.

## KEEP

| Component | Location | What it does | Why it stays |
| --- | --- | --- | --- |
| NES header and CHR paging | `nesticles-core/src/detect.rs`, `codec.rs` | iNES and NES 2.0 sizes, mapper nibble, 2bpp tiles to 64 indexes | Deterministic and tested. Reconstruction must call this, not replace it. |
| Project file | `project.rs` `Project` | Assets, levels, sprites, anim sets, blueprints, behaviors. Serde defaults. No ROM bytes, no API key | The save format Maker already reopens. |
| Sprite and clip model | `anim.rs` `SpriteDef`, `SpriteFrame`, `Part`, `Clip`, `AnimSet`, `present` | Multi-tile parts, per-frame ticks, facing, fallbacks. Rust Play samples clips | This is the one sprite system. R2 must write these records. |
| Documented SMB1 indexes | `pipeline.rs` `smb1_documented`, `place` | Enemy OAM groups. `0xFC` is dropped. Goomba walk is tiles `0x70`–`0x73`; defeat is `0xEF` | Useful evidence after the payload hash matches. Not pixels and not approval. |
| Maker play | `physics/`, `engine/res_1080p/`, `enemy_ai/smb1_nes.rs`, `blueprint.rs`, `terrain.rs`, `transition.rs` | Course sim, SMB1-shaped physics, blueprints | Frozen. Do not add a second physics model. |
| Behavior graphs | `behavior.rs` | Data nodes. `FireReturning` is the projectile that Rust Play steps | Graphs are not scripts. |
| Reference guards | `pipeline.rs` `allow_reference_url`, `openai.rs` fetch, `reference.rs` scan | HTTPS port 443, private-address refusal, capped reads, no script execution, original versus custom | The fetch is sound. Comparison to CHR is missing, which is REPAIR of the workflow, not of the guard. |
| Credentials | `openai.rs`, `keys.rs` | UnMaker key in session, env, or Secret Service. Legacy `keys.json` mode `0600` | Do not add another plaintext store. |
| Empty startup | `app.rs` `fresh_project` | No Seed Spirit autoload. Test `launch_project_has_no_demo_content` | Keep. `demo.rs` remains for tests only. |

## REPAIR

| Component | Location | Defect | Migration |
| --- | --- | --- | --- |
| Extraction job | `pipeline.rs` `job_advance`, `propose_session` | Stages are labels advanced one per UI pump. The session is not in `Project`. Reopen does not restore it. | Persist a reconstruction record. Keep hash, raw tiles, and profile match as pure functions. |
| Review actions | `approve_candidate`, `fail_candidate`, `reinspect`, `ignore_candidate` | Approve requires tile ids and then only changes status. Fail records an attempt. Nothing renders or saves a sprite. | Approve must write `SpriteDef` and `AnimSet`, then the status. |
| Package bridge | `embed_form`, `import_package` | One gray RGBA from the first frame. Every clip points at that same frame id. Palette note in `package_character` says palette is unverified. | Render one picture per `FrameDraft` before this can satisfy R2. |
| Recipes | `Recipe`, `AlphaLibrary` | Loaded on the next extract (`pipeline_ui.rs` `pump_extract`). Not applied as a visible sprite on project open. | A verified recipe should rebuild the sprite without the ROM after the pictures are stored. |
| Guided puzzle | `Puzzle`, `puzzle_view` | Manual index placement. Reference note is text. No preview of assembled CHR. | Becomes the manual branch of an issue, not the primary screen. |
| Reference gallery | `reference.rs`, `pipeline_ui.rs` | Discovers links and can decode one selected image. Does not compare it to CHR. | R4. Do not block the Goomba slice on it. |
| Coverage table | `coverage_for` | Mario and Luigi stay unresolved. Enemy rows say "proposed" when a candidate exists. Palette is always "unverified". | Keep the rows. Drive them from rendered frames, not from the presence of indexes. |
| AI proposals | `openai.rs` `analyze_image`, `validate_ai_proposal` | Optional, consent-gated, PNG only, tile ids checked against CHR count. No live key in this audit. | Stays optional. Never required for Goomba. |

## REPLACE

| Component | Location | Why |
| --- | --- | --- |
| UnMaker screen flow | `studio.rs` `UnmakerView`, `pipeline_ui.rs`, the UnMaker arms of `app.rs` | Home, tiles, review, puzzle, providers, behavior, assembly, and animation are separate panels. The user is expected to assemble. `pipeline_ui.rs` is 1,323 lines and `app.rs` is 6,133. The owner has marked this UI as replaceable. The white studio is specified, not built. |
| In-memory `ExtractUi` as the product record | `pipeline_ui.rs` `ExtractUi` | It cannot be the lifecycle. The project file is the lifecycle. |

REPLACE means a new orchestration module. It does not mean deleting these files in R0 or R1. Maker screens inside `app.rs` stay until an UnMaker path replaces each job and that path is tested.

## RETIRE

| Component | Location | When |
| --- | --- | --- |
| Seed Spirit as a product | `demo.rs` `demo_project` | Already absent from startup. Keep the module until tests stop using it. Do not autoload it. |
| Synthetic fixture as SMB1 proof | `pipeline.rs` `synthetic_fixture`, UnMaker home button | Fine as a parser fixture. It must not be the Goomba acceptance ROM. |
| Second sprite ideas | none in tree | Do not add one. The comment on `SpriteDef` already says it is not a Blueprint. |

## Tests that protect the KEEP set

`codec` and `detect` tests, `anim` round-trip and play tests, `pipeline::tests::smb1_profile_lists_goomba_and_leaves_mario_open`, `pipeline::tests::unknown_hash_does_not_receive_smb1_names`, `reference::` tests, `launch_project_has_no_demo_content`, blueprint and physics play tests. Godot headless tests call `godot/toolchain.rs` `discover` and fail when Godot 4.7.2 is absent. That is a missing tool, not a physics defect.

## Workflow judgment

The repository can identify an SMB1 payload, list documented enemy indexes, decode 2bpp, and play an authored sprite. It cannot, today, turn the Goomba indexes into a saved animation that reopens. That gap is the R2 slice.
