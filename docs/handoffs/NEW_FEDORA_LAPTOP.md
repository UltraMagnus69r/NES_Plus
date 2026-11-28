# New Fedora laptop — Nesticles load handoff

Updated **2026-11-25** from the current Fedora seat (`hostname` `fedora`, user `g`). Use this when standing up Nesticles on a **new** Fedora laptop. GitHub is the source of truth; do not copy the old disk tree by hand.

Repository: https://github.com/UltraMagnus69r/NES_Plus

## What to check out

| Role | Branch | Tip SHA (verified 2026-11-25) | PR |
| --- | --- | --- | --- |
| **Active tip (start here)** | `cursor/r31-responsive-ui-9ff8` | R3.1 product `4619ab6…`; handoff docs on top — **always `git fetch` and use `origin` tip** | [#27](https://github.com/UltraMagnus69r/NES_Plus/pull/27) |
| R3 Slice 3 (owner-accepted) | `cursor/r3-slice3-issue-suggestions-9ff8` | `c64f118ea5f1c14ed9faad5f6f289b1c52405684` | [#26](https://github.com/UltraMagnus69r/NES_Plus/pull/26) |
| Policy / continuity base | `cursor/canonical-github-policy-9ff8` | `4a2bb3cc7afb5a63b612ffff8482ca954703f400` | [#25](https://github.com/UltraMagnus69r/NES_Plus/pull/25) |

`main` (`d583ba8`) is **not** the working branch.

Stack order (newest on top): **#27 → #26 → #25 → … → #1**. Merge or rebase only with owner direction; agents do not force-push or merge to `main`.

Milestone snapshot: R0 / R2 / R2.3 / **R3 accepted**. **R3.1 implemented, not owner-accepted.** **Do not start R4.**

## Clone once (new machine)

Pick **one** project root and keep it forever. On the current Fedora laptop that path is `/home/g/Desktop/NES_Plus`. On a new laptop, use the same idea: one Desktop (or home) folder, no siblings.

```sh
# Example path — replace with the new machine's sole Nesticles root
mkdir -p ~/Desktop
cd ~/Desktop
git clone https://github.com/UltraMagnus69r/NES_Plus.git NES_Plus
cd NES_Plus
git checkout cursor/r31-responsive-ui-9ff8
git fetch origin
git rev-parse HEAD
git rev-parse origin/cursor/r31-responsive-ui-9ff8
# Those two SHAs must match. Product baseline under the tip includes 4619ab6 (R3.1 UI).
```

Auth: use `gh auth login` as **UltraMagnus69r**, then `gh auth setup-git` so HTTPS uses that token (avoids password/ksshaskpass prompts).

If this machine already has a Nesticles folder, **update it in place** — never create a second clone beside it:

```sh
cd /path/to/sole/NES_Plus
./scripts/session-start.sh
# If clean and behind origin:
git pull --ff-only
git rev-parse HEAD
git rev-parse origin/$(git branch --show-current)
```

Those two SHAs must match before claiming sync.

## First-session checklist

1. Confirm local seat (`whoami`, `hostname`, OS). Stop if the path looks like a cloud checkout (`/agent/repos/NES_Plus`).
2. Run `./scripts/session-start.sh` (fetches origin; installs `.githooks` → `.git/hooks`).
3. Read, in order: `AGENTS.md` → `docs/handoffs/CURRENT_PROJECT_STATE.md` → latest `docs/handoffs/daily/` → this file → `docs/handoffs/MULTI_MACHINE_WORKFLOW.md` → `docs/policies/INTELLECTUAL_PROPERTY.md` when adding files.
4. Install Rust toolchain capable of building the workspace (edition/Cargo as in repo).
5. First build may need network for crates; later prefer `--offline` once the cache is warm:

```sh
cargo test -p nesticles
cargo test -p nesticles-core --lib
cargo build -p nesticles
./target/debug/nesticles
```

6. Optional Godot proofs need **Godot 4.7.2 standard** (not .NET), version string `4.7.2.stable.official.ed1daf0bf`. Suggested layout (do **not** commit the binary):

```text
~/.local/share/nesticles/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64
```

Link `godot` / `godot4` on `PATH` when you want the three Godot tests to run. Without 4.7.2 they fail closed.

## ROM (local only)

- Commercial ROMs are **never** in GitHub and must not be committed.
- After the 2026-11-24 Fedora reinstall, `/home/g/Desktop/Roms/smb1.nes` was **absent**.
- Restore a private dump outside the repo when SMB1 proof is needed; set `NESTICLES_ROM` if the path differs.
- ROM-dependent tests skip with a printed line when no dump is present. That is expected, not a CI failure to “fix” by vendoring a ROM.

## What does not travel through Git

- ROM directories and dumps
- `~/.local/share/nesticles/` projects, alpha libraries, managed Godot binary
- API keys (`keys.json`, Secret Service, env)
- `git stash` entries from another machine
- `target/`, `dist/`

## What not to do

- Do **not** start roadmap **R4** (or later functional milestones).
- Do **not** check out / develop from `main` unless the owner says so.
- Do **not** create sibling Nesticles clones or empty `Cursor Repos` duplicates.
- Do **not** force-push, rewrite history for removed images (`maker-tools.png` history finding stays), or use `--no-verify`.
- Do **not** download or commit a ROM to make tests green.
- Do **not** mark R3.1 accepted from automation alone — owner GUI acceptance is still pending.
- Do **not** develop Nesticles on a Cursor cloud Ubuntu checkout; local Fedora/Bazzite seats only.

## Open PR tips (stacked)

Work and review against the **tip** branch `#27` unless the owner names another. Older open PRs `#1`–`#24` are ancestry; leave them alone unless merging the stack by owner request. Policy continuity lives on `#25`; R3 product on `#26`; R3.1 UI on `#27`.

## After every substantial session

Update `CURRENT_PROJECT_STATE.md` + a dated `docs/handoffs/daily/YYYY-MM-DD.md`, run `./scripts/check-repository-safety.sh --staged`, commit, push, confirm `HEAD` == `origin/<branch>`. Or use `./scripts/session-finish.sh --message "..."` after staging any **new** files explicitly (`git add -u` alone skips untracked paths).

More detail: [MULTI_MACHINE_WORKFLOW.md](MULTI_MACHINE_WORKFLOW.md), [CURRENT_PROJECT_STATE.md](CURRENT_PROJECT_STATE.md).
