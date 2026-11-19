# UnMaker-first roadmap

Version: R0, 2026-11-17. Status update: 2026-11-18. See [MILESTONE_STATUS.md](MILESTONE_STATUS.md).

This sequence is separate from historical Phases 0–9.1. Those phases stay in git history and in [packaging/phase9/](../../packaging/phase9/) and [packaging/phase91/](../../packaging/phase91/). They are not renumbered. Historical Phases 10–20 stay suspended. Do not start Phase 10.

Visual build labels R4, R5, R6, and R6.1 are not rows in the table below. R6.1 is the accepted presentation of this application. Roadmap R6 remains "Gameplay content, physics, and behavior authoring" and is not accepted.

The owner approved this order: finish UnMaker Lab before more Maker Engine features. Maker behavior that already works stays. New Maker work waits for the integration gate.

R0 is this audit and these documents. It does not implement R1 or the Goomba slice.

| Step | Name | Done when |
| --- | --- | --- |
| R0 | Architecture rebaseline and code audit | The owner accepts the KEEP/REPAIR/REPLACE decision, the asset lifecycle, and the Goomba plan in this directory. |
| R1 | UnMaker studio foundation | One white UnMaker shell can import a ROM, show investigation progress, and review candidates. The extraction session stays in memory until R2 writes a sprite. The owner asked for this visual theme. |
| R2 | Verified NES extraction and automatic reconstruction | The Goomba slice in [GOOMBA_VERTICAL_SLICE.md](../architecture/GOOMBA_VERTICAL_SLICE.md) passes on the owner's ROM: animated preview, approve, save, reopen, ROM no longer required. |
| R3 | Intelligent issue resolution and local learning | One unresolved issue is presented with a suggestion. Accept, skip, and reject are stored. A rejected grouping is not offered again unchanged. |
| R4 | Reference intelligence and AI integration | A selected reference can sit beside the reconstruction. AI runs only with a credential and an explicit send. |
| R5 | Complete animation and creation studio | The user can edit frames, clips, forms, timing, origins, and anchors, and see the preview update. |
| R6 | Gameplay content, physics, and behavior authoring | An approved picture can receive one existing Blueprint and one behavior graph. No second physics model. |
| R7 | Permanent library, packaging, and Maker handoff | One package round-trips into Maker as a placeable animated object. |
| Gate | UnMaker-to-Maker integration acceptance | The owner plays that object in Maker from a reopened project. |
| After | Historical Maker roadmap | Phases 10–20 may resume. They do not start before the gate. |

Phase 9.1 remains unaccepted. Its test candidate does not satisfy R2.

R2 assembles one verified SMB1 Goomba. The walk row `70 71 72 73` is one metasprite, and the second pose is that metasprite mirrored for 8 frames, which is the `FrameCounter` bit 3 test in this payload. Defeat is tile `EF` twice, with the right copy flipped, and it does not loop. Approval writes `SpriteDef` and `AnimSet` into the project and does not store the cartridge. The baked colors are the review palette and are not a verified original NES palette. When the normal investigation finishes, Discover lists that Goomba as a reconstructed card. Review is where its Walk and Defeat / Stomped states are inspected and saved. The owner accepted R2 and R2.3 on November 18, 2026. R3 is authorized. Acceptance does not mark the review palette as an original NES palette, and it does not mark the machine-specific ROM test as passed on a machine that does not have that ROM.

A later investigation, not part of R2, needs a graphics inventory of the whole CHR: reconstructed assets, partial assets, identified but unresolved assets, unidentified graphics, shared graphics, and unused or unconfirmed graphics. That inventory should group related tiles instead of treating every tile as its own sprite. It is groundwork for R3 and R4. R2.2 does not build it and does not reconstruct the rest of SMB1.

R1.3 is a shared-interface correction. UnMaker and Maker use one white visual system. The old Source Tools destination is no longer a separate shell. Its screens open from the stage that owns the job: source tiles and audio from Discover, notes and providers from Investigate, the behavior graph from Create, and library, export, and overworld from Save. Those screens are the existing tools. They do not by themselves reconstruct a sprite.

R1.2 is a review-usability correction on the studio. Source-tile previews can use display palettes. Those colors are for review and are not a verified original palette. A reference URL that returns HTTP 403 is described as a site block, with a way to continue, import a local image, or fetch a direct image. R1.2 does not reconstruct a sprite. R2 is separate from this correction. The owner later accepted R2.

R1.1 corrects the window. The R1 test candidate started fullscreen. The owner replaced that requirement: Nesticles launches in a decorated, resizable window. UnMaker Lab and Maker Engine share it. Fullscreen is optional and starts only from the menu or F11. Escape leaves fullscreen. Older release notes that say the app opened fullscreen describe those builds, not this requirement.
