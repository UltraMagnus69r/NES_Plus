# Current project state

Updated 2026-11-24 on the Fedora laptop (`hostname` `fedora`, user `g`) after the fresh Fedora reinstall continuity update.

## Branch and commit

- Repository: https://github.com/UltraMagnus69r/NES_Plus
- Sole Fedora checkout: `/home/g/Desktop/NES_Plus` (obsolete path `/home/g/Desktop/CURSOR/nes_plus` must not be recreated)
- Working branch: `cursor/canonical-github-policy-9ff8`
- Starting tip for this continuity session: `99073eb86dca60ab244de305034c8f0b0ddc5833` (`Remove four images that have no redistribution record.`)
- Pull request https://github.com/UltraMagnus69r/NES_Plus/pull/25 remains open and not merged. This continuity commit extends that branch.
- `main` is not the working branch.
- R2.2 pull request https://github.com/UltraMagnus69r/NES_Plus/pull/23 was not merged by prior sessions.
- Published R2.2 test candidate: tag `r22-character-presentation`, file `Nesticles-R2.2-CharacterPresentation-linux-x86_64.tar.gz`, SHA256 `6649d34bd58d441952e2407d871371c0f028ca9ca4652d14b8cd856bb6c69cc1`
- Remote tag `r2.3-design-language-12d7ce898` points at `12d7ce8981b27f7e53c7cc6cdf6a4ce9b2cb4f7a`

## Milestones

The acceptance list is `docs/roadmaps/MILESTONE_STATUS.md`. It records owner acceptance of R2 and R2.3 on 2026-11-18, and R3 as authorized and incomplete. R4 and later functional milestones are not accepted. Visual labels R4, R5, R6, and R6.1 are presentation labels, not those roadmap milestones. Phase 9.1 is not accepted. Phase 10 has not started.

This continuity session does not accept or revoke a milestone.

## Architecture

Nesticles is a native Rust desktop application (eframe / egui). UnMaker Lab extracts and authors. Maker Engine places parts and plays courses. They share one decorated window and one project JSON. Sprite and animation data stay in `SpriteDef` and `AnimSet`. The project must not store a ROM or an API key.

The Goomba reconstruction lives in `crates/nesticles-core/src/reconstruct.rs`. Colors are the review palette, not a verified original NES palette. Owner acceptance of R2 does not change that.

## Environment (Fedora, 2026-11-24)

- OS: Fedora Linux 45 (KDE Plasma Desktop Edition), hostname `fedora`, user `g`
- Local Cursor development only. Cloud agents prohibited.
- Godot 4.7.2 standard verified: `~/.local/share/nesticles/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64` reports `4.7.2.stable.official.ed1daf0bf`; `godot` and `godot4` on `PATH`. Also present under `~/.config/craft-anything/engines/godot/`. Godot binary is not in git.
- Owner ROM `/home/g/Desktop/Roms/smb1.nes` is **absent** after reinstall. ROM-dependent owner-path proof is limited until restored locally. Do not download or commit a ROM.
- Redundant empty clones under `Desktop/Cursor Repos/nes_plus*` were removed. Do not recreate them.

## R3 status (verified in source, not accepted)

Present:

- `Project.issue_decisions`
- Issue fingerprinting (`cand:{asset_id}|…`)
- Accept / skip / reject decision handling
- Rejection-hide for an unchanged grouping
- Investigate shows an eligible `issues::Issue` with capsule actions

Still incomplete:

- Roadmap finished condition is not owner-accepted
- `IssueCard.suggestion` remains `None` (Investigate uses `issues::Issue` instead of filling that card field)
- Mario and Luigi stay unresolved

**Precise next unfinished R3 acceptance requirement:** continue R3 until one unresolved issue is presented with a suggestion, accept/skip/reject are stored, a rejected grouping is not offered again unchanged, and the owner accepts the milestone — without starting R4. Do not implement that product work in a continuity-only session.

## Known issues

- Mario and Luigi are unresolved.
- `maker-tools.png` remains reachable in older commits; history safety scan can still report it. Do not rewrite history.
- Godot preview still does not play Phase 9 clips (separate from the three headless proofs).
- A package's older readers still see the first frame when they ignore `pictures`.
- Pre-reinstall local R2.3 stash is gone on this laptop. Do not look for it here.

## Build and launch

```sh
cd /home/g/Desktop/NES_Plus
cargo test --offline -p nesticles
cargo test --offline -p nesticles-core --lib
cargo build -p nesticles --offline
./target/debug/nesticles
```

On Bazzite, use the sole clone under `/home/UltraMagnus69r` and the same cargo commands. ROM tests skip when `NESTICLES_ROM` and both owner paths are absent.

## Next priority

After this continuity push’s remote SHA matches HEAD: continue **R3** from `cursor/canonical-github-policy-9ff8`. Do not start roadmap R4. Do not rewrite history. Read `docs/handoffs/daily/2026-11-24.md` before editing.

## Acceptance

This continuity update does not accept a milestone. R2 and R2.3 acceptance remains the 2026-11-18 record in `MILESTONE_STATUS.md`.
