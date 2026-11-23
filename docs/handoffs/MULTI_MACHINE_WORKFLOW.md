# Multi-machine workflow

Machines:

- Fedora laptop: checkout `/home/g/Desktop/CURSOR/nes_plus`, user `g`.
- Bazzite desktop: home `/home/UltraMagnus69r`. Clone the same repository there. Do not copy the Fedora tree by hand.

Repository: https://github.com/UltraMagnus69r/NES_Plus

Cloud agent checkouts, including user `ubuntu` and `/agent/repos/NES_Plus`, are not development machines. Stop.

## Start

```sh
cd /path/to/NES_Plus
./scripts/session-start.sh
```

The script prints the user, host, OS, origin, branch, HEAD, and short status. It fetches `origin`. It does not change your files. `git fetch` downloads new commits and leaves the checkout where it was.

Then read, in order:

1. `AGENTS.md`
2. `docs/handoffs/CURRENT_PROJECT_STATE.md`
3. The latest file in `docs/handoffs/daily/`
4. `docs/handoffs/DECISION_LOG.md` when the task touches workflow, assets, or architecture
5. `docs/policies/INTELLECTUAL_PROPERTY.md` when the task adds files

If the script says the branch is behind origin and the working tree is clean, read the incoming commits and update with:

```sh
git pull --ff-only
```

If it says the branches have diverged, stop. Do not `git reset --hard`. Decide which commits to keep with the owner.

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

A test that prints `ROM-dependent test skipped` did not prove SMB1. Set `NESTICLES_ROM` to a dump that stays outside the repo if you need that test. Do not commit that file.

## During the session

Keep product commits separate from policy commits. Do not commit `local/`, `keys.json`, or anything the safety script flags.

```sh
./scripts/check-repository-safety.sh --staged
```

The pre-commit hook installed by session-start runs the same check. Do not use `--no-verify`.

## Finish

1. Update `docs/handoffs/CURRENT_PROJECT_STATE.md`.
2. Add a session section to `docs/handoffs/daily/YYYY-MM-DD.md`.
3. Add a decision when the session changed a rule or a structure.
4. Stage the files you mean to publish. `git add -u` does not add untracked files. Stage new source on purpose.
5. Run the safety check.
6. Commit. One concern per commit.
7. Push and compare SHAs:

```sh
git push -u origin HEAD
git fetch origin
git rev-parse HEAD
git rev-parse origin/$(git branch --show-current)
```

Those two SHAs must match. Until they do, the session is not synchronized.

`./scripts/session-finish.sh --message "..."` stages tracked modifications (`git add -u`), runs the safety check, commits, pushes, and compares the SHAs. It does not stage brand-new files. Stage those first. It does not force-push.

## What does not travel through Git

- `/home/g/Desktop/Roms/` and any other ROM directory.
- `~/.local/share/nesticles/` projects and alpha libraries.
- API keys in Secret Service, the environment, or `keys.json`.
- `git stash` entries. A stash on the Fedora laptop is not on the Bazzite desktop.
- `target/` and `dist/`.

The other machine reproduces the app from the clone, then points `NESTICLES_ROM` at its own local dump when a ROM test is required.
