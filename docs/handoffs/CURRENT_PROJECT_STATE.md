# Current project state

Updated **2026-11-28** (America/Chicago) on the Fedora laptop (hostname `g`, user `g`) after Roadmap R4 reference intelligence implementation. Product work is on `cursor/r4-reference-intelligence`, stacked on `cursor/r31-responsive-ui-9ff8`.

## Branch and commit

- Repository: https://github.com/UltraMagnus69r/NES_Plus
- Sole Fedora checkout: `/home/g/Desktop/NES_Plus`
- Working branch: `cursor/r4-reference-intelligence` (confirm live tip with `git rev-parse` / session-start after push)
- Parent R3.1.1 tip: `cursor/r31-responsive-ui-9ff8` @ `f98ee87e5dcfdee0d6dd7839fa541caac47a2458` — PR [#27](https://github.com/UltraMagnus69r/NES_Plus/pull/27)
- Parent R3 Slice 3: `cursor/r3-slice3-issue-suggestions-9ff8` @ `c64f118ea5f1c14ed9faad5f6f289b1c52405684` — PR [#26](https://github.com/UltraMagnus69r/NES_Plus/pull/26)
- Continuity / policy PR: [#25](https://github.com/UltraMagnus69r/NES_Plus/pull/25) @ `4a2bb3c` remains open
- New Fedora laptop load steps: [NEW_FEDORA_LAPTOP.md](NEW_FEDORA_LAPTOP.md)
- `main` is not the working branch

## Milestones

| Milestone | Status |
| --- | --- |
| R2 / R2.3 | Owner-accepted (2026-11-18) |
| R3 | **Owner-accepted 2026-11-24** — intelligent issue resolution / Slice 3 suggestions. |
| R3.1 | Implemented; **visual acceptance failed** (clipping / overflow / weak reflow). |
| R3.1.1 | **Correction implemented.** **Owner-approved for progression** into R4 (2026-11-28). Does not invent mouse/4K evidence. |
| R4 | **Implemented** — reference beside reconstruction; AI credential + explicit send. **Owner acceptance pending.** See [R4_REFERENCE_INTELLIGENCE.md](../roadmaps/R4_REFERENCE_INTELLIGENCE.md). |
| R5+ | Not authorized. |

## R4 implementation (this session)

- Core `ref_intel` module: asset-linked references, deterministic compare notes, AI send gate
- Project persistence: `asset_references`, `ai_assist_records` (serde defaults for old projects)
- UnMaker Reference compare tool from Discover / Review
- Local import + direct image URL association; nearest-neighbor zoom
- Opt-in AI with payload confirmation; mock path; no auto-send; no R3 override
- Evidence: `docs/handoffs/evidence/r4-reference/`

Preserved:

- Design Language 2.0 / R6.1 presentation tokens
- R3 suggestion generation and `issue_decisions` semantics
- Maker game coordinates
- No ROM committed; no R5; no history rewrite

## Environment

- Fedora Linux 44 KDE Plasma, user `g`, path `/home/g/Desktop/NES_Plus`, Wayland
- Panel: 1920×1080 @ scale 1
- Godot `4.7.2` on PATH via managed install
- Owner SMB1 dump used for GUI evidence only (not committed)

## Next priority

Owner conducts **R4 acceptance** against the checklist in `docs/roadmaps/R4_REFERENCE_INTELLIGENCE.md`. Do not start R5. Do not merge unfinished stacked PRs to `main`.

## Acceptance

- R3: owner-accepted (2026-11-24).
- R3.1.1: owner-approved for progression (not a claim of mouse/4K re-verification).
- R4: **pending owner acceptance**.
