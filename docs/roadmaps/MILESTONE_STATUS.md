# Milestone status

Recorded November 18, 2026, from the owner's acceptance of R2 and R2.3.
Updated **2026-11-28** (America/Chicago) after R4.1 reference zoom + saved-creation restore correction. R3.1.1 is owner-approved for progression; physical 4K / independent mouse acceptance evidence is not invented.

| Milestone | Status |
| --- | --- |
| R0 — Architecture rebaseline | Accepted before this date. |
| R1 — UnMaker studio foundation | Unchanged by this decision. The studio shell is in the product. This record does not add a new R1 acceptance mark. |
| R2 — Goomba reconstruction | Accepted by the owner on November 18, 2026. |
| R2.3 — Design Language 2.0 | Accepted by the owner on November 18, 2026. |
| R3 — Intelligent issue resolution and local learning | **Owner-accepted on 2026-11-24** (session date of the acceptance instruction). Covers completed R3 functionality including evidence-backed Investigate suggestions (Slice 3) and decision persistence. Acceptance does **not** claim that previously unverified mouse-driven actions were independently re-tested in that acceptance moment. |
| R3.1 — Responsive UI and resolution independence | **Implemented**; owner visual acceptance failed. See R3.1.1. |
| R3.1.1 — Responsive UI correction | **Correction implemented** on `cursor/r31-responsive-ui-9ff8`. **Owner-approved for progression** into R4 (2026-11-28 assignment). Approval does **not** invent physical 4K GUI tests or independently automated mouse acceptance that was never recorded. Framebuffer evidence remains under `docs/handoffs/evidence/r311-preview/`. |
| R4 — Reference intelligence and AI integration | **Implemented**; **owner acceptance failed** on reference zoom + Save→Open restore. **R4.1 correction implemented** on the same branch/PR. See `docs/roadmaps/R4_REFERENCE_INTELLIGENCE.md`. Owner re-acceptance pending. |
| R5 and later functional milestones | Not accepted. Not authorized. |
| Historical Phase 9.1 | Not accepted. |
| Historical Phases 10–20 | Suspended. |

## Visual labels are not roadmap numbers

R4, R5, R6, and R6.1 on the launch scripts and UI notes are visual build labels on the R2.3 architecture. Roadmap R4 is the functional milestone above. The accepted presentation baseline remains R6.1 chrome. Do not roll it back and do not open another visual direction in place of R4.

## What acceptance does not verify

- The Goomba unit tests try `/home/g/Desktop/Roms/smb1.nes` and then `/home/UltraMagnus69r/Desktop/Nesticle/ROMS/Super Mario Bros. (World)/Super Mario Bros. (World).nes`. They fail only when neither file can be read. A screenshot is still not that test.
- Framebuffer captures and `NESTICLES_*` tour variables are not a mouse pass.
- Drag-to-resize, save/reopen of a resized course, and Bazzite launch were reported by the visual handoff. This acceptance session does not repeat those as new passes unless a command in the session report says so.
- Review colors remain a review palette.
- Mario and Luigi stay unresolved.
- R3 owner acceptance does not, by itself, convert earlier launch-only GUI evidence into a mouse-driven pass.
- R3.1.1 progression approval does not convert framebuffer-only evidence into a mouse or 4K pass.

## Accepted presentation

UnMaker Lab and Maker Engine are two workspaces in one decorated native window. The chrome includes the glossy rounded controls, selected and default icons, scenic backgrounds, the Discover gallery and inspector, the Maker parts browser and tool sidebar, an empty course rather than a sample level, and the existing NES cell grid. `ui_assets` embeds those images and can read replacements from `~/Desktop/Nesticle/Assets` without overwriting files already there. The Maker wallpaper is decoration. It is not written into course cells.

R3.1 adapts density and panel layout across display sizes while preserving this Design Language 2.0 presentation. R4 adds reference compare inside that chrome without a redesign.
