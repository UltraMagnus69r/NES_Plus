# Current project state

Updated 2026-11-23 on the Fedora laptop (`hostname` `laptop`, user `g`).

## Branch and commit

- Repository: https://github.com/UltraMagnus69r/NES_Plus
- Published development tip before this policy commit: `d7c39ace11670696d4536e50e0bb3ed1693fc294` on `cursor/r3-issue-decisions`
- Policy branch: `cursor/canonical-github-policy-9ff8`, based on that tip
- The policy commit SHA is whatever `git rev-parse HEAD` prints after the push matches `origin/cursor/canonical-github-policy-9ff8`. This file is inside that commit, so it does not invent the SHA.
- `main` is not the working branch.
- R2.2 pull request https://github.com/UltraMagnus69r/NES_Plus/pull/23 is based on `cursor/r2-goomba-reconstruction-9ff8` and was not merged by this session.
- Published R2.2 test candidate: tag `r22-character-presentation`, file `Nesticles-R2.2-CharacterPresentation-linux-x86_64.tar.gz`, SHA256 `6649d34bd58d441952e2407d871371c0f028ca9ca4652d14b8cd856bb6c69cc1`
- Remote tag `r2.3-design-language-12d7ce898` points at `12d7ce8981b27f7e53c7cc6cdf6a4ce9b2cb4f7a`. This session did not rebuild or re-download that archive.

## Milestones

The acceptance list is `docs/roadmaps/MILESTONE_STATUS.md`, already on the parent commit. It records owner acceptance of R2 and R2.3 on 2026-11-18, and R3 as authorized and incomplete. R4 and later functional milestones are not accepted. Visual labels R4, R5, R6, and R6.1 are presentation labels, not those roadmap milestones. Phase 9.1 is not accepted. Phase 10 has not started.

This policy session did not re-test that acceptance and does not revoke it.

## Architecture

Nesticles is a native Rust desktop application (eframe / egui). UnMaker Lab extracts and authors. Maker Engine places parts and plays courses. They share one decorated window and one project JSON. Sprite and animation data stay in `SpriteDef` and `AnimSet`. The project must not store a ROM or an API key.

The Goomba reconstruction lives in `crates/nesticles-core/src/reconstruct.rs`. Colors are the review palette, not a verified original NES palette. Owner acceptance of R2 does not change that.

## What this session verified

The ROM-dependent Goomba round-trip ran on this laptop because `/home/g/Desktop/Roms/smb1.nes` was present. A machine without that file, the Bazzite path, or `NESTICLES_ROM` skips the test and does not download a ROM.

Mouse operation of the R6.1 chrome was not exercised in this session. Launch hooks are not a mouse pass.

A local git stash from an earlier unfinished R2.3 edit remains on this laptop only (`WIP R2.3 design language, unfinished, excluded from the IP-policy commit`). GitHub already has a later R2.3 commit, `12d7ce8`. Do not pop that stash onto this branch.

## Known issues

- Mario and Luigi are unresolved.
- R3 is authorized and not complete.
- Godot preview does not play Phase 9 clips. The Godot parity tests fail on this laptop when Godot 4.7.2 is absent. That failure is the existing fail-closed behavior. This session did not install Godot.
- A package's older readers still see the first frame when they ignore `pictures`.

## Build and launch

```sh
cd "$HOME/Desktop/CURSOR/nes_plus"    # Fedora
cargo test --offline -p nesticles
cargo test --offline -p nesticles-core --lib
cargo build -p nesticles --offline
./target/debug/nesticles
```

On Bazzite, use the clone under `/home/UltraMagnus69r` and the same cargo commands. ROM tests skip when `NESTICLES_ROM` and both owner paths are absent.

## Next priority

Continue R3 from `cursor/canonical-github-policy-9ff8` after the remote SHA matches HEAD. Do not start roadmap R4. Do not roll back the R6.1 presentation. Read `docs/handoffs/daily/2026-11-23.md` before editing.

## Acceptance

This policy commit does not accept a milestone. R2 and R2.3 acceptance remains the 2026-11-18 record in `MILESTONE_STATUS.md`.
