# ADR 0001 — Retain the core and rebuild UnMaker orchestration

Status: proposed for owner acceptance of R0. Not implemented.

## Decision

Choose **B**. Keep `nesticles-core` as the engine. Rebuild UnMaker's session, investigation, and interface so a reconstruction becomes a `SpriteDef` plus an `AnimSet` in the project file. Leave Maker Engine's placement and play path in place and frozen.

## Options

### A. Keep patching the current UnMaker screens

`ExtractUi` in [pipeline_ui.rs](../../crates/nesticles/src/pipeline_ui.rs) holds the session in memory. [approve_candidate](../../crates/nesticles-core/src/pipeline.rs) only sets `ReviewStatus::Approved`. [embed_form](../../crates/nesticles-core/src/pipeline.rs) paints the first frame with a gray ramp and [import_package](../../crates/nesticles-core/src/pipeline.rs) points every clip at that one picture. More buttons on this path will not produce a reopened animation.

Feasible for a small fix. Poor fit for the issue queue, live preview, and the white studio. `app.rs` is 6,133 lines and already owns both workspaces.

### B. Retain the core and rebuild UnMaker orchestration

Codecs, the project schema, sprites, clips, blueprints, physics, and the reference-URL guard are already data. The missing piece is a reconstruction record that survives save and renders every frame. Maker can keep consuming `Project`.

This is the recommendation. Regression risk stays in UnMaker. Maker tests stay. Data migration is additive: old projects omit the new record and open as they do now.

### C. New application, migrating modules

A second binary would fork save files, play, and packaging. The current crates already separate core from the editor. A new app does not remove the frame-collapse bug unless the same model work is done. Cost is higher and Maker compatibility is worse.

## Consequences

R1 adds a focused UnMaker module and stops growing `app.rs` for reconstruction. R2 renders Goomba from CHR indexes into `SpriteDef` frames. The parchment UnMaker screens stay until the owner asks for the white studio. No Phase 10 work. No second sprite system.
