# Current project state

Updated **2026-11-28** (America/Chicago) on the Fedora laptop (hostname `g`, user `g`) after the R3.1.1 Discover reconstruction-preview follow-up. Product work is on `cursor/r31-responsive-ui-9ff8`.

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
| R3.1.1 | **Correction + Discover preview follow-up implemented** (2026-11-28). Automated + framebuffer GUI evidence. **Not owner-accepted.** |
| R4 | Unauthorized. Not started. |

## R3.1.1 Discover preview follow-up (this session)

Owner-reported after the first R3.1.1 correction:

- Compact Discover kept card selection but lost the reconstruction preview (gallery consumed remaining height; CollapsingHeader was unreachable).
- Wide Discover side inspector overflowed the content rectangle (item spacing + soft shadow + forced child mins).

Implemented:

- Explicit **View Preview** / **Hide Preview** + constrained Reconstruction preview window on compact/laptop Discover
- `discover_split_widths` / clip / no soft-shadow side inspector that stays inside the content width
- Shared `discover_inspector` body; `StudioState.discover_preview_open` survives resize
- Regression tests in `responsive::tests`; evidence under `docs/handoffs/evidence/r311-preview/`

Preserved:

- Design Language 2.0 / R6.1 presentation tokens
- R3 suggestion generation and `issue_decisions` semantics
- Maker `EDITOR_CELL_PX` / `pointer_cell` game coordinates
- No ROM committed; no R4; no history rewrite

## Environment

- Fedora Linux 44 KDE Plasma, user `g`, path `/home/g/Desktop/NES_Plus`, Wayland
- Panel: 1920×1080 @ scale 1 (maximized client captured 1920×1006)
- Godot `4.7.2.stable.official.ed1daf0bf`
- Owner SMB1 dump present locally for GUI evidence only (not committed; not under `/home/g/Desktop/Roms/smb1.nes`)

## Next priority

Owner conducts **actual GUI acceptance** of R3.1.1 (including Discover preview follow-up) on the laptop. Do not mark R3.1 / R3.1.1 accepted from automation alone. Do not start R4.

## Acceptance

- R3: owner-accepted (2026-11-24).
- R3.1 / R3.1.1: **pending owner acceptance**.
