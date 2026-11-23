# Intellectual property and repository content

This policy is mandatory for every Nesticles commit, pull request, release archive, and development machine. It does not by itself prove who owns a file. When a check cannot tell, the file stays out of GitHub until a person reviews it.

The official repository is https://github.com/UltraMagnus69r/NES_Plus.

## Allowed in GitHub

- Rust source, Cargo manifests, and the lockfile.
- Build, packaging, and session scripts.
- Original Nesticles icons and interface artwork made for this project.
- Permissive third-party material with a recorded license, attribution, and source.
- Synthetic test fixtures that are not commercial game data.
- Architecture, API, roadmap, handoff, and decision documents.
- Release checksums and version text. The release archive itself stays on GitHub Releases, not as a new committed dump, except the Alpha 1 archive already in history. See the audit note in the decision log.

## Not allowed in GitHub

Commercial game ROMs, cartridge dumps, BIOS files, extracted sprite sheets, copyrighted backgrounds, protected character graphics, commercial music, extracted audio, sound effects without redistribution rights, copyrighted animations, maps that embed those graphics, project files that embed those pixels, unlicensed commercial screenshots, and archives of any of those.

The rule follows the bytes, not the name. Renaming `.nes` to `.dat`, base64-encoding a header, wrapping a dump in zip, or pasting tiles into JSON does not make the file allowed.

Protected material stays on the machine that already has it. Development must continue from a clone without that material. ROM-dependent tests read `NESTICLES_ROM` or the local path documented below. They do not download a commercial ROM.

## Classifications

| Class | Meaning | Commit? |
| --- | --- | --- |
| ORIGINAL | Created for Nesticles and approved by the owner for redistribution. | Yes. |
| PERMISSIVE | Third-party material whose license allows redistribution, with the record below. | Yes, with the record. |
| LOCAL_PROTECTED | A user-supplied commercial ROM or an extraction from one. | Never. |
| UNKNOWN | Rights not established. | No, until a person changes the class. |

A picture that resembles a commercial game is not automatically LOCAL_PROTECTED, and it is not automatically ORIGINAL. If the rights are not written down, the class is UNKNOWN.

### Record for permissive material

- Source URL or distribution.
- Creator.
- License name and version.
- Attribution text.
- What the license lets us redistribute.
- Where the license file lives in this repository.

### Current records

| Path | Class | Record |
| --- | --- | --- |
| `crates/nesticles/assets/LiberationSerif-Regular.ttf` | PERMISSIVE | Liberation Fonts, SIL Open Font License 1.1. Copyright 2010 Google Corporation and 2012 Red Hat, Inc. Reserved font names are in `crates/nesticles/assets/LICENSE-Liberation.txt`. Source: https://github.com/liberationfonts |
| `crates/nesticles/assets/LiberationSerif-Bold.ttf` | PERMISSIVE | Same license file. |
| `crates/nesticles/assets/icon.png` | ORIGINAL | Nesticles application icon already tracked before this policy. |
| `crates/nesticles-player/pkg/nesticles_player_bg.wasm` | ORIGINAL | Generated player bundle. It is tracked so the desktop build can embed the current player. It is not a ROM. Rebuild it from the player crate rather than treating it as source art. |
| `packaging/alpha1/Nesticles-Alpha1-linux-x86_64.tar.gz` | ORIGINAL | Historical test-candidate archive already in git. Its members are the Nesticles executable, the icon, and text. It is listed in the safety allowlist so the size check does not treat that known archive as a new unknown binary. Do not add further release archives to git. |

## Where local assets live

Do not move or delete an existing ROM collection as part of policy work.

On the Fedora laptop the owner's cartridge dumps stay in `/home/g/Desktop/Roms/`. The Bazzite path tried by the tests is `/home/UltraMagnus69r/Desktop/Nesticle/ROMS/Super Mario Bros. (World)/Super Mario Bros. (World).nes`. Tests also honor `NESTICLES_ROM` when that variable points at a file outside the repository. Neither path is created or filled by git.

Optional untracked drop folders inside a checkout, if someone creates them:

- `local/`
- `local-roms/`
- `local-assets/`
- `extracted/`

Those directories are gitignored. They are not created by this policy and they are not copied between machines by git.

Saved Nesticles projects and the alpha library stay in the OS data directory (`~/.local/share/nesticles/`). They are not repository content. A project JSON that contains extracted commercial pixels is LOCAL_PROTECTED and must not be committed.

## Tests and CI

ROM-independent tests use synthetic bytes or original fixtures. They run on a clean clone.

ROM-dependent tests read the local file. If it is missing they print a skip line and return. They must not fetch a ROM. GitHub Actions must not receive a ROM. The workflow `.github/workflows/repository-safety.yml` runs the content check only.

Known SMB1 validation checksums, for a file the owner already has, are documentation only:

- File SHA256: `0b3d9e1f01ed1668205bab34d6c82b0e281456e137352e4f36a9b2cfa3b66dea`
- Headerless payload SHA256: `fcb6a0ef3a20c19b356005fbb21dc8009563b1cb5a9aaebc8e9386b4a8c5912e`

The file itself is not in git.

## Releases

A GitHub Release may contain the Nesticles executable, the original icon, and documentation. It must not contain a ROM, a credential, a user project, or extracted commercial art. Checksums are recorded in the release notes after a real download.

## What the automatic check does

`scripts/check-repository-safety.sh` and `scripts/repository_safety.py` inspect bytes.

They flag:

- Cartridge and ROM extensions, including inside zip and tar members.
- Files that start with an iNES, NSF, UNIF, or FDS header, whatever the extension is.
- Game Boy logo bytes in a non-container file.
- Text, JSON, and project files that contain a raw NES or NSF header, or a base64 blob that decodes to one.
- Common secret patterns. The match is reported. The value is not printed.
- Tracked or staged files of 2 MB or more that are not in the reviewed allowlist.
- Archives that cannot be opened.

They do not flag an ELF, PNG, gzip, wasm, RIFF, or Ogg file merely because the NES header bytes also occur later in that container. The Nesticles executable contains the four-byte iNES constant as code.

A clean run is not a copyright opinion.

## Audit note, 2026-11-23

A byte scan of reachable git blobs found no iNES, NSF, UNIF, or FDS header and no secret pattern that the checker recognizes. It did find one file outside the reviewed large-file list:

- `crates/nesticles/assets/ui/backgrounds/maker-tools.png`
- blob `ed8d3c7a161665a5fe1ba981b1a74893f472f43a`
- commit `cef7367742849d4397f7cc78db67d7407edf498b`
- 2,013,115 bytes, PNG, 684×2048

The picture is editor decoration in a glossy platform-game style. This repository has no provenance record that establishes redistribution rights for it. Classification: UNKNOWN. It was already on GitHub before this policy. This session does not delete it and does not rewrite history.

The design reference images added in `12d7ce8981b27f7e53c7cc6cdf6a4ce9b2cb4f7a` also have no license record. `docs/design/references/UNMAKER_LAB_V2_MOCKUP.jpg` names Mario, Luigi, and Koopa and shows a character labeled Goomba. Classification: UNKNOWN. The owner reviews the rights. A later commit can remove or document the files. Removing the blobs from GitHub, if the owner wants that, is a separate history rewrite on every machine.

The checker does not decide copyright by looking at a picture. GitHub Actions runs the worktree scan, so this existing file keeps that check red until it is classified or removed.

## If a protected file is committed

1. Do not push.
2. If the commit is still only local, remove the file in a new commit. Do not amend a pushed commit. Do not force-push.
3. If the file already reached GitHub, stop. Tell the owner. Do not rewrite history from an agent session. Rewriting shared history is a separate operation because every machine has the old commits.
4. Leave the local copy of the owner's ROM where it is. Deleting the owner's dump is not remediation.
5. Record the path and the commit id in the daily handoff. Do not paste the file's bytes or a secret.

## Contributor duty

Read this file before adding art, audio, fixtures, or project JSON. Classify the file. If the class is not ORIGINAL or a recorded PERMISSIVE entry, do not `git add` it. Run `./scripts/check-repository-safety.sh --staged` before every commit. A hook installed by `./scripts/session-start.sh` runs that check. Do not skip the hook to get a commit through.
