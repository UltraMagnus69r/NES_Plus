# Current project state

Updated **2026-11-27** (America/Chicago) on the Fedora laptop (hostname `g`, user `g`) after R3.1.1 responsive UI correction. Product work is on `cursor/r31-responsive-ui-9ff8`.

## Branch and commit

- Repository: https://github.com/UltraMagnus69r/NES_Plus
- Sole Fedora checkout: `/home/g/Desktop/NES_Plus`
- Working / active tip: `cursor/r31-responsive-ui-9ff8` (confirm live tip with `git rev-parse` / session-start after push)
- Parent R3 Slice 3: `cursor/r3-slice3-issue-suggestions-9ff8` @ `c64f118ea5f1c14ed9faad5f6f289b1c52405684` — PR [#26](https://github.com/UltraMagnus69r/NES_Plus/pull/26)
- R3.1 / R3.1.1 PR: [#27](https://github.com/UltraMagnus69r/NES_Plus/pull/27) stacked on #26
- Continuity / policy PR: [#25](https://github.com/UltraMagnus69r/NES_Plus/pull/25) @ `4a2bb3c` remains open
- New Fedora laptop load steps: [NEW_FEDORA_LAPTOP.md](NEW_FEDORA_LAPTOP.md)
- `main` is not the working branch

## Milestones

| Milestone | Status |
| --- | --- |
| R2 / R2.3 | Owner-accepted (2026-11-18) |
| R3 | **Owner-accepted 2026-11-24** — intelligent issue resolution / Slice 3 suggestions. |
| R3.1 | Implemented; **visual acceptance failed** (clipping / overflow / weak reflow). |
| R3.1.1 | **Correction implemented** (2026-11-27). Automated + framebuffer GUI evidence. **Not owner-accepted.** |
| R4 | Unauthorized. Not started. |

## R3.1.1 (this session)

Root causes: width-only mode kept Standard chrome on short laptop clients; gallery card mins could overflow the row; inspector split did not prove fit; filters did not wrap; shell chips overflowed ~800×500; Maker tool capsules were fixed-width; shot harness applied geometry too late for Wayland.

Implemented:

- `density()` short-client densification through Compact
- `gallery_fit` / `discover_split_widths` that never overflow content
- Discover filter wrap + density-based shell/Maker chrome
- Extended responsive unit tests; evidence under `docs/handoffs/evidence/r311-responsive/`
- Docs: `docs/roadmaps/R3_1_RESPONSIVE_UI.md`

Preserved:

- Design Language 2.0 / R6.1 presentation tokens
- R3 suggestion generation and `issue_decisions` semantics
- Maker `EDITOR_CELL_PX` / `pointer_cell` game coordinates
- No ROM download; no R4; no history rewrite

## Environment

- Fedora Linux 44 KDE Plasma, user `g`, path `/home/g/Desktop/NES_Plus`, Wayland
- Panel: 1920×1080 @ scale 1 (maximized client captured 1920×1006)
- Godot `4.7.2.stable.official.ed1daf0bf`
- Owner ROM `/home/g/Desktop/Roms/smb1.nes` still absent

## Next priority

Owner conducts **actual GUI acceptance** of R3.1.1 on the laptop (and preferably one larger display). Do not mark R3.1 / R3.1.1 accepted from automation alone. Do not start R4.

## Acceptance

- R3: owner-accepted (2026-11-24).
- R3.1 / R3.1.1: **pending owner acceptance**.
