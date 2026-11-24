# Multi-machine workflow

Machines:

- Fedora laptop: sole Nesticles checkout `/home/g/Desktop/NES_Plus`, user `g`, hostname `fedora` (Fedora Linux 45 KDE after the 2026-11-24 reinstall). The previous path `/home/g/Desktop/CURSOR/nes_plus` is obsolete. Do not create redundant clones or sibling Nesticles folders on this machine.
- Bazzite desktop: home `/home/UltraMagnus69r`. Keep one established Nesticles checkout under that home. Do not assume the exact project subdirectory until it has been verified on Bazzite. Do not copy the Fedora tree by hand.

Repository: https://github.com/UltraMagnus69r/NES_Plus

All Nesticles development, building, and application testing happens through Cursor on these local seats. Cloud agent checkouts, including user `ubuntu` and `/agent/repos/NES_Plus`, are not development machines. Stop.

When GitHub is ahead of the local checkout and the worktree is clean, update **this exact directory** with a fast-forward pull. Never create another clone or worktree to receive the update. Never discard local work merely to match GitHub.

## Start

```sh
cd /home/g/Desktop/NES_Plus   # Fedora; on Bazzite use that machine's sole checkout
./scripts/session-start.sh
```

The script prints the user, host, OS, origin, branch, HEAD, and short status. It fetches `origin`. It does not change your files. `git fetch` downloads new commits and leaves the checkout where it was. It installs the local safety hooks from `.githooks/` into `.git/hooks/`.

Then:

1. Verify you are on a local Fedora or Bazzite seat (not a cloud path).
2. Inspect local changes (`git status --short`).
3. Note ahead/behind from session-start.
4. Identify the authoritative development branch from `docs/handoffs/CURRENT_PROJECT_STATE.md`.
5. If behind and clean, read the incoming commits, then `git pull --ff-only` in this directory.
6. Confirm local and remote SHAs when claiming alignment.
7. Read, in order:
   1. `AGENTS.md`
   2. `docs/handoffs/CURRENT_PROJECT_STATE.md`
   3. The latest file in `docs/handoffs/daily/`
   4. `docs/handoffs/DECISION_LOG.md` when the task touches workflow, assets, or architecture
   5. `docs/policies/INTELLECTUAL_PROPERTY.md` when the task adds files
8. Begin only authorized work.

If the script says the branches have diverged, stop. Do not `git reset --hard`. Decide which commits to keep with the owner.

If the script says there is no upstream, the branch has not been pushed. Push it at the end of the session or check out the branch named in `CURRENT_PROJECT_STATE.md`.

`main` is not the working branch.

## Build and test

```sh
cargo test --offline -p nesticles
cargo test --offline -p nesticles-core --lib
cargo build -p nesticles --offline
./target/debug/nesticles
```

`--offline` requires a populated Cargo cache. The first build on a machine may need a normal `cargo test -p nesticles` so crates can download. Do not vendor a ROM to make tests pass.

Godot preview and the three Godot proofs need Godot 4.7.2 standard on the machine (managed path or `PATH`). They fail closed when that editor is missing. Do not commit the Godot binary.

A test that prints `ROM-dependent test skipped` did not prove SMB1. Set `NESTICLES_ROM` to a dump that stays outside the repo if you need that test. Do not commit that file. After the Fedora reinstall, `/home/g/Desktop/Roms/smb1.nes` was absent; restore it locally when owner-path SMB1 proof is required.

## During the session

Keep product commits separate from policy commits. Do not commit `local/`, `keys.json`, or anything the safety script flags.

```sh
./scripts/check-repository-safety.sh --staged
```

The pre-commit hook installed by session-start runs the same check. Do not use `--no-verify`.

## Finish

1. Review changes.
2. Run applicable tests.
3. Update `docs/handoffs/CURRENT_PROJECT_STATE.md`.
4. Add a session section to `docs/handoffs/daily/YYYY-MM-DD.md`.
5. Add a decision when the session changed a rule or a structure.
6. Stage the files you mean to publish. `git add -u` does not add untracked files. Stage new source on purpose.
7. Run the safety check.
8. Commit. One concern per commit.
9. Push and compare SHAs:

```sh
git push -u origin HEAD
git fetch origin
git rev-parse HEAD
git rev-parse origin/$(git branch --show-current)
```

Those two SHAs must match. Until they do, the session is not synchronized.

10. Return a comprehensive handoff.

`./scripts/session-finish.sh --message "..."` stages tracked modifications (`git add -u`), runs the safety check, commits, pushes, and compares the SHAs. It does not stage brand-new files. Stage those first. It does not force-push.

## What does not travel through Git

- `/home/g/Desktop/Roms/` and any other ROM directory.
- `~/.local/share/nesticles/` projects, alpha libraries, and the managed Godot editor binary.
- API keys in Secret Service, the environment, or `keys.json`.
- `git stash` entries. A stash on the Fedora laptop is not on the Bazzite desktop.
- `target/` and `dist/`.

The other machine reproduces the app from the clone, then points `NESTICLES_ROM` at its own local dump when a ROM test is required.
