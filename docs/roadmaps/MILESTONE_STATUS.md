# Milestone status

Recorded November 18, 2026, from the owner's acceptance of R2 and R2.3.

| Milestone | Status |
| --- | --- |
| R0 — Architecture rebaseline | Accepted before this date. |
| R1 — UnMaker studio foundation | Unchanged by this decision. The studio shell is in the product. This record does not add a new R1 acceptance mark. |
| R2 — Goomba reconstruction | Accepted by the owner on November 18, 2026. |
| R2.3 — Design Language 2.0 | Accepted by the owner on November 18, 2026. |
| R3 — Intelligent issue resolution and local learning | Authorized and in progress. Decisions persist on `Project.issue_decisions`, and Investigate can record one issue. The milestone is not complete. |
| R4 and later functional milestones | Not accepted. Not started as roadmap work. |
| Historical Phase 9.1 | Not accepted. |
| Historical Phases 10–20 | Suspended. |

## Visual labels are not roadmap numbers

R4, R5, R6, and R6.1 on the launch scripts and UI notes are visual build labels on the R2.3 architecture. Roadmap R6 is still "Gameplay content, physics, and behavior authoring" and is not accepted. The accepted presentation baseline is R6.1. Do not roll it back and do not open another visual direction in place of R3.

## What acceptance does not verify

- The Goomba unit test that reads `/home/g/Desktop/Roms/smb1.nes` stays failed or blocked on a machine without that file. A local ROM used for a screenshot is not that test.
- Framebuffer captures and `NESTICLES_*` tour variables are not a mouse pass.
- Drag-to-resize, save/reopen of a resized course, and Bazzite launch were reported by the visual handoff. This acceptance session does not repeat those as new passes unless a command in the session report says so.
- Review colors remain a review palette.
- Mario and Luigi stay unresolved.

## Accepted presentation

UnMaker Lab and Maker Engine are two workspaces in one decorated native window. The chrome includes the glossy rounded controls, selected and default icons, scenic backgrounds, the Discover gallery and inspector, the Maker parts browser and tool sidebar, an empty course rather than a sample level, and the existing NES cell grid. `ui_assets` embeds those images and can read replacements from `~/Desktop/Nesticle/Assets` without overwriting files already there. The Maker wallpaper is decoration. It is not written into course cells.
