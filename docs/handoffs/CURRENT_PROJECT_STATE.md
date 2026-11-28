# Current project state

Updated **2026-11-28** (America/Chicago) on the Fedora laptop (hostname `g`, user `g`) after **R4.1** reference zoom + saved-creation restore correction. Product work is on `cursor/r4-reference-intelligence`, stacked on `cursor/r31-responsive-ui-9ff8`.

## Branch and commit

- Repository: https://github.com/UltraMagnus69r/NES_Plus
- Sole Fedora checkout: `/home/g/Desktop/NES_Plus`
- Working branch: `cursor/r4-reference-intelligence` (confirm live tip with `git rev-parse` / session-start after push)
- PR: https://github.com/UltraMagnus69r/NES_Plus/pull/28
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
| R4 | **Implemented.** **Owner acceptance failed** on reference zoom + Save→Open restore. |
| R4.1 | **Correction implemented** — independent reference/recon zoom; durable association + Open a Saved Creation restore. Owner re-acceptance pending. |
| R5+ | Not authorized. |

## R4.1 correction (this session)

Root causes:

1. Compare panes called `ResponsiveLayout::preview_fit`, which re-clamped zoomed sizes into the panel (and capped upscale). Larger imported references looked stuck while tiny reconstructions still appeared to zoom. One shared slider also coupled the panes.
2. References were often keyed to ephemeral Discover `candidate-*` ids. Save wrote them, but reopen had no session, left Maker/Create without selecting the sprite, and `reconstruction_rgba` required a live ROM session — so the association looked missing.

Fixes:

- Independent `ref_pane_zoom` / `recon_pane_zoom`; display size = `native × zoom`; scroll when oversized
- Rewire candidate→sprite on Approve & Save; orphan recovery on open; associate to durable sprite when published
- `restore_saved_creation` after Open a Saved Creation / `NESTICLES_PROJECT`
- Composite saved sprite frames when no extraction session

Evidence: `docs/handoffs/evidence/r41-zoom-restore/`. Owner `~/.local/share/nesticles/creations/goomba.json` was **not** mutated (SHA unchanged).

Preserved: Design Language 2.0 / R6.1 chrome, R3 decisions, Maker behavior, no R5, no ROM commit.

## Environment

- Fedora Linux 44 KDE Plasma, user `g`, path `/home/g/Desktop/NES_Plus`, Wayland
- Panel: 1920×1080 @ scale 1
- Godot `4.7.2` on PATH via managed install
- Owner SMB1 dump used only when needed for live tour (not committed)

## Next priority

Owner conducts **R4 / R4.1 acceptance** against the checklist in `docs/roadmaps/R4_REFERENCE_INTELLIGENCE.md` (especially mouse-driven Reference zoom + Save → quit → Open a Saved Creation). Do not start R5. Do not merge unfinished stacked PRs to `main`.

## Acceptance

- R3: owner-accepted (2026-11-24).
- R3.1.1: owner-approved for progression (not a claim of mouse/4K re-verification).
- R4: **owner acceptance failed**; R4.1 correction ready for re-acceptance.
