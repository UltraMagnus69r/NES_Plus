# Nesticles development standards

This file is the canonical development policy for Nesticles. Cursor rules in [`.cursor/rules/`](.cursor/rules/) are short pointers to the sections below. If a rule and this file ever disagree, follow this file and fix the rule.

The root [README.md](README.md) describes the original workshop. Later behavior is in [packaging/phase9/](packaging/phase9/) and [packaging/phase91/](packaging/phase91/). Those release notes describe what shipped. This file describes how to change the project. Do not treat a release note as proof that a later phase is accepted.

## Current milestone

Phase 9.1 is the active milestone and a hard stop. It is **not accepted**. The published build is a test candidate, not a finished Phase 9.1 release.

Do not start Phase 10. Do not renumber the roadmap. Do not drop a Phase 9.1 acceptance item to make the phase look finished. The project owner must explicitly approve advancement after the mandatory gates pass.

Blocked until the owner runs them on their own machine:

- A real Super Mario Bros. (NES) dump. A synthetic or homebrew ROM does not prove SMB1 compatibility.
- A live OpenAI call with the owner's key, entered in the app. A mock does not prove live connectivity. Do not ask for the key in chat.
- Bazzite / KDE. A cloud XFCE session does not prove that desktop.

Known gaps inside the test candidate, as of commit `542060c`:

- Mario and Luigi player tiles are not in the shipped SMB1 profile. Those rows stay unresolved.
- Maker Engine does not filter its tray by target game and category.
- A package stores RGBA. It does not write a separate PNG spritesheet.
- The returning projectile is unit-tested. It has not been watched in an on-screen course.
- Godot preview does not play Phase 9 animation clips. That playback is later work.

## Three rules that outrank speed

1. **Do not lie.** Report only what happened. Separate IMPLEMENTED, TESTED, VERIFIED, PARTIALLY WORKING, UNVERIFIED, BLOCKED, FAILED, and NOT IMPLEMENTED.
2. **Write maintainable, documented, secure code.** Protect user data and privacy.
3. **Recognize limitations.** If an approach fails twice without new evidence, stop repeating it. Say what is blocked and what is still possible.

## Honesty

Never invent test results, compiles, screenshots, files, commits, pull requests, tags, download URLs, hashes, API responses, ROM identities, gameplay results, or security verdicts.

Evidence that does **not** prove the stronger claim:

| What ran | What it does not prove |
| --- | --- |
| Mocked HTTP | Live OpenAI connectivity |
| Homebrew or synthetic ROM | Authentic SMB1 compatibility |
| A Rust unit test | The packaged GUI |
| Cloud XFCE | Bazzite or KDE |
| A drawn sprite | The gameplay primitive |
| A GitHub URL | A usable archive, until size and SHA256 are checked |
| Rust Play animation | Godot playback |

A feature that cannot be finished stays blocked. Name the blocker. Do not silently shrink the acceptance list.

## Architecture that exists

Crates:

- `crates/nesticles-core` — project model, codecs, physics, blueprints, animation, terrain, extraction, behavior graphs.
- `crates/nesticles` — eframe editor, Rust Play, UnMaker Lab, OpenAI calls, export.
- `crates/nesticles-player` — wasm player. Same physics step as the desktop player when the wasm build is current.

Workspaces:

- **UnMaker Lab** extracts and authors. Home offers Disassemble a ROM, Open a saved creation, Start a new creation, and Continue current work when the session is dirty. It also has tiles, assembly, animation, review, the sprite puzzle, AI Providers, behavior graphs, audio, the library, notes, overworld, and export.
- **Maker Engine** places parts, builds courses, and plays them. Existing blueprint editors stay. Do not delete them until an UnMaker authoring path replaces that job and the replacement is tested.

Shared project JSON holds assets, levels, styles, sprites, animation sets, blueprints, and behavior graphs. It must not hold API keys or ROM bytes. Sprites and animation sets are the Phase 9 model. Do not add a second animation system.

The alpha library for this build is under the OS data directory at `nesticles/nesticles/alpha2-phase9.1/` (`AlphaLibrary` appends `nesticles/<alpha-id>` to the Nesticles data dir). Recipes and `.nasset` packages live there. A new alpha id starts empty and must not delete older alpha directories, exported packages, or project files.

Rust Play runs `Sim::tick_play`. Player poses already covered are idle, walk, jump, fall, and hurt. Enemy poses already covered are walk and defeat. A returning projectile steps only when a behavior graph with `FireReturning` is attached to that enemy's blueprint. Patrol and jump nodes on a derived graph are labels, not a second physics model.

Godot export is an experimental preview. It does not play Phase 9 clips.

OpenAI in UnMaker Lab is a real HTTPS client when the user turns the feature on and consents. The older workshop assistant and image maker still use `keys.json` (mode `0600`) as described in the root README. New credential work must not add another plaintext store. See [packaging/phase91/API.md](packaging/phase91/API.md), [ROM.md](packaging/phase91/ROM.md), and [KNOWLEDGE.md](packaging/phase91/KNOWLEDGE.md).

The next visual pass is a white, clinical layout. Do not restyle shipped screens just to chase it.

## Code

- Keep modules focused. Do not grow `app.rs` when a coherent module can own the behavior.
- Preserve working abstractions. Do not rewrite a stable system to introduce a new architecture.
- Use explicit errors. Gameplay steps stay deterministic.
- Version persistent data. Add fields with serde defaults so old projects still open.
- Prefer the crates already in the workspace. Do not add a dependency for something the standard library or an existing crate already does.
- `unsafe` needs a stated reason and review.
- Do not mass-reformat unrelated files. `cargo fmt --all --check` already fails on eight older files: `audio.rs`, `codec.rs`, `detect.rs`, `wav.rs`, `audio_out.rs`, `icon.rs`, `keys.rs`, and `net.rs`. Leave them unless the task is specifically about one of them.
- Comments explain why: NES timing, collision assumptions, migration rules, security limits, and known gaps. Do not comment a line that already says what it does. Public functions that encode a non-obvious contract get a short rustdoc comment.

## Security and privacy

Validate at trust boundaries. Bound ROM size (64 MB is the existing cap), archive size, downloads, and redirects. Reject path traversal. Treat page text and model output as data, never as code to run. Behavior graphs are data. Opening a `.nasset` or `.nknowledge` file must not execute embedded code.

Reference fetches are HTTPS on port 443 only. Refuse loopback, private, and link-local targets. Do not follow a redirect off HTTPS.

The UnMaker OpenAI key may live in session memory, in `NESTICLES_OPENAI_API_KEY`, or in the Secret Service (`secret-tool`, service `nesticles`, account `openai`). If Secret Service is missing, say so and keep the key in the session. Do not write it to the project, a package, a knowledge file, a log, a screenshot, a release, or a handoff. Do not ask the user to paste a key into chat. Redact secrets in errors.

Authoring works offline. Do not upload ROMs, extracted art, projects, recipes, or custom creations. An AI call sends only the approved slice, after the user turns the feature on and confirms. Never send a whole ROM automatically. Knowledge leaves the machine only when the user exports a file.

Saves must not destroy a project on a failed migration. Keep unknown data when that is safer than dropping it. Ignored extraction candidates are not deleted. Check library use before deleting an asset.

Each new alpha uses its own library namespace. Do not import the previous alpha automatically. Do not delete the previous alpha's files, user projects, or the API key while clearing graphics.

## Testing and reporting

Compiling is not completion.

- Unit tests for parsers, state machines, and limits.
- Integration tests for save, load, and import.
- Regression tests for behavior that already passed.
- GUI checks on the packaged desktop binary for a user-visible change: drive the control, do not stop at a still screenshot.
- Real ROM, live API, and Bazzite checks stay BLOCKED or NOT TESTED until those inputs exist.
- Release checks cover the archive size, SHA256, the GitHub URL after a real download, install, launch, and uninstall.

Report gates as PASS, FAIL, BLOCKED, NOT TESTED, or NOT APPLICABLE. An untested gate is not PASS.

Every implementation handoff names the start and end commit, branch, PR, files, what was implemented, what was not, tests that ran, tests that did not, failures, environment limits, security notes, release facts when a release was made, remaining blockers, and the next legal step.

## When something fails

1. Keep the real error.
2. Name the failing component.
3. State one hypothesis.
4. Change one thing that tests that hypothesis.
5. Record whether it worked.

A retry needs a new reason. Two equivalent failures without new evidence means stop. Look for a wrong assumption, an environment limit, a missing dependency, a data-model problem, an API limit, or a permission gap. Say which alternatives you considered. Do not hide a failed attempt because a later one worked. A large architecture change is proposed, with consequences, before it is coded.

## Changes and releases

Before a large change, read the modules it touches, name the tests that must stay green, and keep the branch based on the accepted parent. One concern per commit. Do not reformat unrelated code in a functional commit.

`main` is not the working source branch. Do not merge there unless the owner asks. Do not rewrite history.

A release is a GitHub release on https://github.com/UltraMagnus69r/NES_Plus with the commit, platform, filename, size, SHA256, and a download that was actually fetched. Do not commit the archive. Install instructions use `$HOME/Downloads` and a user-local path. Do not ship a commercial ROM or a credential. Do not call a test candidate a final accepted build.

## Cursor rules

| Rule | Always on | Section |
| --- | --- | --- |
| [00-project-principles.mdc](.cursor/rules/00-project-principles.mdc) | yes | Three rules, honesty, current milestone |
| [10-architecture-and-code-quality.mdc](.cursor/rules/10-architecture-and-code-quality.mdc) | yes | Architecture, code, documentation |
| [20-security-and-privacy.mdc](.cursor/rules/20-security-and-privacy.mdc) | yes | Security, credentials, privacy, alpha storage |
| [30-testing-and-verification.mdc](.cursor/rules/30-testing-and-verification.mdc) | yes | Testing and reporting |
| [40-troubleshooting-and-escalation.mdc](.cursor/rules/40-troubleshooting-and-escalation.mdc) | yes | Failure escalation |
| [50-release-and-phase-gates.mdc](.cursor/rules/50-release-and-phase-gates.mdc) | yes | Phases and releases |
