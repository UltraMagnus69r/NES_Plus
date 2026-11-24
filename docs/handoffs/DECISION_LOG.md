# Decision log

## D-001 — Local machines only

- Date: 2026-11-23
- Problem: A cloud agent checkout can drift from the machines that actually build and run Nesticles.
- Alternatives: Allow cloud agents with a read-only mirror. Develop only in the cloud.
- Decision: No Cursor cloud agent develops Nesticles. Coding, building, and testing happen on the Fedora laptop or the Bazzite desktop.
- Rationale: The owner approved this. GUI and ROM checks need the local seat. A path such as `/agent/repos/NES_Plus` under user `ubuntu` is not a local machine.
- Consequences: `scripts/session-start.sh` exits if it detects that cloud path. Agents stop instead of continuing there.
- Status: Accepted by the owner. In force.

## D-002 — GitHub is the source of truth

- Date: 2026-11-23
- Problem: Two computers cannot share uncommitted work.
- Alternatives: Copy directories by hand. Keep a second remote.
- Decision: https://github.com/UltraMagnus69r/NES_Plus is the only canonical remote for source, architecture, original graphics, tools, docs, history, and safe evidence.
- Rationale: A fresh clone must be enough to build and continue, apart from local-only ROMs, secrets, and user data.
- Consequences: Distributable work is pushed at the end of a session. `git fetch` alone does not update the working tree.
- Status: Accepted by the owner. In force.

## D-003 — End of session is a verified push

- Date: 2026-11-23
- Problem: A commit that stays on one laptop is invisible to the other machine.
- Alternatives: Push only at milestones. Commit locally and sync later without a check.
- Decision: A substantial session updates the handoff, commits, pushes the development branch, and checks that `origin` has the same SHA. No force-push. No silent merge to `main`.
- Rationale: The owner wants every session transferable.
- Consequences: If the push fails, the session is not synchronized. The recovery is the error from `git push`, then a normal push after fixing credentials or conflicts. History is not rewritten to make the push succeed.
- Status: Accepted by the owner. In force.

## D-004 — Compare with GitHub before changing machines

- Date: 2026-11-23
- Problem: The second machine can start from a stale branch.
- Alternatives: Assume the last chat message is current. Hard-reset to origin on every start.
- Decision: Start of session fetches, compares ahead and behind, and stops when the branches have diverged. Update a clean tree with `git pull --ff-only` only after looking at the incoming commits. Do not use hard reset as the normal sync.
- Rationale: A hard reset deletes local commits that were never pushed.
- Consequences: Divergence is a stop, not an automatic repair.
- Status: Accepted by the owner. In force.

## D-005 — No protected third-party IP in GitHub

- Date: 2026-11-23
- Problem: Cartridge data and extracted art were at risk of being committed with source.
- Alternatives: Rely on filenames. Allow dumps in a private folder of the same repo.
- Decision: Commercial ROMs, extracted commercial art, commercial audio, and project files that embed those bytes never go to GitHub, regardless of extension or encoding. See `docs/policies/INTELLECTUAL_PROPERTY.md`.
- Rationale: The owner made this nonnegotiable. Renaming and encoding do not create a redistribution right.
- Consequences: Safety checks scan bytes. UNKNOWN assets stay untracked. A leak that has been pushed is not fixed by a force-push from an agent.
- Status: Accepted by the owner. In force.

## D-006 — ROMs stay local

- Date: 2026-11-23
- Problem: SMB1 tests needed a real cartridge and must not download one.
- Alternatives: Vendor a ROM. Skip the tests forever. Fail the build on every machine that lacks the file.
- Decision: Tests read `NESTICLES_ROM`, then `/home/g/Desktop/Roms/smb1.nes`, then the Bazzite path recorded in `OWNER_SMB1_PATHS`. If none can be read they print a skip line and return. CI never receives a ROM. Checksums in the IP policy identify a file the owner already has. They are not the file.
- Rationale: Bazzite and a clean clone must still build and run the ROM-independent suite.
- Consequences: A green test log on a machine without the ROM does not prove SMB1 behavior. The skip line has to be read.
- Status: Accepted by the owner. In force.

## D-007 — UnMaker-first

- Date: 2026-11-17, restated 2026-11-23
- Problem: Maker feature work was ahead of a verified extraction path.
- Alternatives: Continue historical Phases 10–20. Build both products without a gate.
- Decision: The active sequence stays R0–R7 in `docs/roadmaps/UNMAKER_FIRST_ROADMAP.md`. Acceptance is the list in `docs/roadmaps/MILESTONE_STATUS.md`, which already records owner acceptance of R2 and R2.3 on 2026-11-18 and R3 as authorized and incomplete. This policy does not revoke that record and does not accept R4 or Phase 10.
- Rationale: The owner approved UnMaker-first work. GitHub already contains that later acceptance record. This session adds repository rules on top of it.
- Consequences: New product work follows `MILESTONE_STATUS.md`. This policy commit does not implement gameplay or reconstruction.
- Status: Accepted by the owner. In force.

## D-008 — UnMaker and Maker stay separate

- Date: 2026-11-17, restated 2026-11-23
- Problem: Extraction and course editing were collapsing into one screen model.
- Alternatives: One workspace that both disassembles and plays. A second sprite system for UnMaker.
- Decision: UnMaker Lab extracts and authors. Maker Engine places and plays. They share the window and the project JSON. Sprites and clips stay in `SpriteDef` and `AnimSet`.
- Rationale: The project already has this split. A second animation system would fork saved data.
- Consequences: New character work extends the existing model. Maker blueprint editors stay until a tested UnMaker path replaces them.
- Status: Accepted. In force.

## D-009 — Native Rust desktop

- Date: 2026-11-23
- Problem: A visual rebuild could be read as permission to replace the UI toolkit.
- Alternatives: A browser front end. A second engine for presentation.
- Decision: Nesticles stays a native eframe/egui desktop application. The window stays decorated. Fullscreen is optional and user-initiated.
- Rationale: The owner asked to preserve the Rust desktop architecture. Godot remains an experimental preview and does not play Phase 9 clips.
- Consequences: UI work uses egui. No new UI framework for the policy or for later visual work unless the owner opens that decision.
- Status: Accepted by the owner. In force.

## D-010 — Remove the four UNKNOWN images from the tree

- Date: 2026-11-23
- Problem: Four tracked images had no provenance record. One is the Maker tools backdrop. Three are design references. They show recognizable commercial-game characters and objects.
- Alternatives: Leave them tracked. Add them to the safety allowlist. Replace the backdrop with a lookalike. Rewrite Git history.
- Decision: Remove the four files from the current branch. Do not allowlist them. Do not draw a replacement that copies a commercial game. The tools panel already has a solid fill, so the controls stay. Keep `docs/design/NESTICLES_DESIGN_LANGUAGE_V2_0.md`. Do not rewrite history.
- Rationale: The repository rule forbids uploading protected third-party material without verified redistribution rights. These files do not have that record. Absence from the current tree is not absence from old commits.
- Consequences: GitHub Actions history scan can still report `maker-tools.png` from commit `cef7367`. That finding stays visible. Other UI images were not reclassified by this decision.
- Status: In force. History rewrite is not authorized.

## D-011 — Sole Fedora checkout after reinstall

- Date: 2026-11-24
- Problem: After the Fedora reinstall, Nesticles lived at `/home/g/Desktop/NES_Plus`, while docs still named `/home/g/Desktop/CURSOR/nes_plus`. Empty sibling clones under `Desktop/Cursor Repos/` risked confusion.
- Alternatives: Keep both paths. Recreate `CURSOR/nes_plus`. Allow multiple Desktop clones.
- Decision: The sole Fedora Nesticles checkout is `/home/g/Desktop/NES_Plus`. The `CURSOR/nes_plus` path is obsolete. Do not create redundant Nesticles project folders. When GitHub is ahead, update that directory in place with a fast-forward pull. Empty redundant clones were removed.
- Rationale: The owner required one clean project root and GitHub-driven multi-machine sync without hand-copied trees.
- Consequences: Agents and docs must use `/home/g/Desktop/NES_Plus`. Bazzite keeps one checkout under `/home/UltraMagnus69r` without assuming the subdirectory until verified there.
- Status: Accepted by the owner. In force.

## D-012 — Godot 4.7.2 is the verified local editor pin

- Date: 2026-11-24
- Problem: Nesticles Godot proofs require a pinned standard editor. After reinstall, Godot was missing; 4.4.1 was briefly installed and is the wrong pin.
- Alternatives: Commit Godot into the repo. Accept 4.4.1. Leave proofs failing forever.
- Decision: Keep Godot 4.7.2 standard (not .NET) as a user-local tool at `~/.local/share/nesticles/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64`, reachable as `godot` / `godot4` on `PATH` when linked. Do not commit the executable. Verified version string: `4.7.2.stable.official.ed1daf0bf`.
- Rationale: Matches `nesticles_core::godot::toolchain` and keeps the binary out of GitHub.
- Consequences: Machines without 4.7.2 still fail the three Godot proofs closed. Bazzite must install its own matching editor when those proofs are required there.
- Status: Accepted by the owner. In force.
