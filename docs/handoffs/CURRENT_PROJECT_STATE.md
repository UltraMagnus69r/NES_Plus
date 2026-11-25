# Current project state

Updated **2026-11-25** (America/Chicago) on the Fedora laptop (`hostname` `fedora`, user `g`) after a GitHub sync audit and new-laptop handoff write-up. Product tip unchanged since R3.1 implementation (2026-11-24).

## Branch and commit

- Repository: https://github.com/UltraMagnus69r/NES_Plus
- Sole Fedora checkout: `/home/g/Desktop/NES_Plus`
- Working / active tip: `cursor/r31-responsive-ui-9ff8` (R3.1 UI product commit `4619ab6f1a029bbc18426c12fcdc682dbc1ca54d`; 2026-11-25 handoff docs committed on top — confirm live tip with `git rev-parse` / session-start)
- Parent R3 Slice 3: `cursor/r3-slice3-issue-suggestions-9ff8` @ `c64f118ea5f1c14ed9faad5f6f289b1c52405684` — PR [#26](https://github.com/UltraMagnus69r/NES_Plus/pull/26)
- R3.1 PR: [#27](https://github.com/UltraMagnus69r/NES_Plus/pull/27) stacked on #26
- Continuity / policy PR: [#25](https://github.com/UltraMagnus69r/NES_Plus/pull/25) @ `4a2bb3c` remains open
- New Fedora laptop load steps: [NEW_FEDORA_LAPTOP.md](NEW_FEDORA_LAPTOP.md)
- `main` is not the working branch

## Milestones

| Milestone | Status |
| --- | --- |
| R2 / R2.3 | Owner-accepted (2026-11-18) |
| R3 | **Owner-accepted 2026-11-24** — intelligent issue resolution / Slice 3 suggestions. Mouse-driven Investigate actions were **not** independently re-verified solely due to acceptance. |
| R3.1 | **Active / implemented** — responsive UnMaker + Maker layouts. **Not owner-accepted.** |
| R4 | Unauthorized. Not started. |

## R3.1 (this session)

Implemented:

- `ResponsiveLayout` / `LayoutMode` in `crates/nesticles/src/responsive.rs`
- UnMaker adaptive nav, hero, Discover gallery/inspector, Investigate density
- Maker adaptive tools width / parts strip; play surface fits live client area
- Window inner-size clamp for monitor changes; `MIN_INNER` 800×500
- Docs: `docs/roadmaps/R3_1_RESPONSIVE_UI.md`
- Focused responsive unit tests

Preserved:

- Design Language 2.0 / R6.1 presentation tokens
- R3 suggestion generation and `issue_decisions` semantics
- Maker `EDITOR_CELL_PX` / `pointer_cell` game coordinates
- No ROM download; no R4; no history rewrite of `maker-tools.png`

## Environment

- Fedora 45 KDE, user `g`, path `/home/g/Desktop/NES_Plus`
- Godot `4.7.2.stable.official.ed1daf0bf`
- Owner ROM `/home/g/Desktop/Roms/smb1.nes` still absent

## Sync status (2026-11-25)

- After 2026-11-25 handoff push(es): local tip matches `origin/cursor/r31-responsive-ui-9ff8` (ahead 0 / behind 0); confirm with session-start
- Stack bases on origin unchanged: policy `4a2bb3c`, R3 `c64f118`; R3.1 product commit `4619ab6` remains in history under the tip
- No stash entries

## Next priority

Owner conducts **actual GUI acceptance** of R3.1 on the 1366×768 laptop (and preferably one larger display). Do not mark R3.1 accepted from automation alone. Do not start R4. On a new Fedora laptop, follow [NEW_FEDORA_LAPTOP.md](NEW_FEDORA_LAPTOP.md).

## Acceptance

- R3: owner-accepted (2026-11-24).
- R3.1: **pending owner acceptance**.
