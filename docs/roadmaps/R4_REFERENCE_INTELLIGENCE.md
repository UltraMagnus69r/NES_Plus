# R4 — Reference Intelligence and AI Integration

Roadmap milestone R4 from [UNMAKER_FIRST_ROADMAP.md](UNMAKER_FIRST_ROADMAP.md).

**Canonical acceptance:** A selected reference can sit beside the reconstruction. AI runs only with a credential and an explicit send.

## Status

| Item | State |
| --- | --- |
| Implementation | Implemented on `cursor/r4-reference-intelligence` |
| Automated tests | Passing (core + desktop) |
| GUI framebuffer evidence | R4 under `docs/handoffs/evidence/r4-reference/`; R4.1 under `docs/handoffs/evidence/r41-zoom-restore/` |
| Live paid AI call | Not required for automated tests; offline/mock path covered |
| Owner acceptance | **Failed on first pass** — reference zoom + Save→Open restore. **R4.1 correction implemented;** owner re-acceptance pending |

**R4:** IMPLEMENTED, OWNER ACCEPTANCE FAILED — REFERENCE ZOOM AND SAVE/RESTORE  
**R4.1:** CORRECTION IMPLEMENTED (not yet owner-accepted)  
**R5:** NOT AUTHORIZED

R3.1.1 was **owner-approved for progression** into R4. That approval does not invent mouse-driven or physical 4K GUI evidence that was never recorded for R3.1.1.

## Existing architecture (KEEP / EXTEND)

| Area | Decision | Notes |
| --- | --- | --- |
| `nesticles_core::reference` | KEEP | URL scan, HTTPS limits, HTTP 403 honesty, local `RefImage`, attribution |
| `nesticles::openai` | EXTEND | Keyring/env/session credentials; vision analyze; redaction |
| Investigate / Discover reference gallery | KEEP | Session fetch + import still available |
| R3 `issue_decisions` | KEEP | Accept / Skip / Reject and fingerprints unchanged |
| Design Language 2.0 / R6.1 chrome | KEEP | No redesign |
| Competing reference DB / second AI stack | REPLACE refused | Single project association list + existing OpenAI client |

## Implemented design

### Persistence (`ref_intel` + `Project`)

- `Project.asset_references: Vec<AssetReference>` — stable `asset_id`, title, notes, source kind, URL/hint, confidence, optional embedded PNG (base64), dimensions, active flag.
- `Project.ai_assist_records: Vec<AiAssistRecord>` — suggestion-only review history. Never writes `SpriteDef` / `AnimSet` by itself.
- Old projects without these fields still open (serde defaults).
- Embedded bytes live in the owner's project file. They are not GitHub fixtures. Commercial ROM art is not committed.

### UnMaker Reference workspace

- Open from Discover (**Open reference**) or Review (**Open reference workspace**) for the selected card.
- Import local image (native picker) or direct image URL (async fetch; page fetch remains available).
- Side-by-side Reference / Reconstruction with **independent** zoom sliders (1–24), nearest-neighbor textures.
- Display size is `native × zoom` (R4.1). Zoomed panes scroll when larger than the panel; they are **not** re-clamped by `preview_fit` (that hid reference zoom for larger images).
- Compact / short clients use a denser chrome so the pair stays reachable; below ~720 pt width the compare uses two compact columns.
- Associations prefer durable sprite ids. Approve & Save rewires ephemeral Discover candidate ids onto the published sprite. Home → Open a Saved Creation restores UnMaker selection and opens Reference compare when embedded references exist; orphan session ids are recovered onto the saved sprite.
- Deterministic intelligence notes: Observed / Derived / Uncertain (dimensions, scale mismatch, opacity, sampled mismatch). No invented NES frames.

### AI security model

- Workspace works with AI disabled and with no credential.
- `gate_ai_send` requires credential + user confirmation of payload + explicit Send.
- Opening a project, selecting a card, importing a reference, or resizing does not send.
- Payload description names image/note sizes and states the ROM is excluded.
- Errors and responses are redacted against the session key.
- `NESTICLES_AI_MOCK=1` records a mock suggestion without network use (tests / offline demos).
- AI output is labeled suggestion-only. It does not override R3 decisions.

## Test results (implementation session)

| Gate | Result |
| --- | --- |
| `cargo test -p nesticles-core --lib` | PASS — 190 after R4.1 (was 187; +3 zoom/rewire/restore) |
| `cargo test -p nesticles` | PASS — 68, 1 ignored |
| `cargo build -p nesticles --release` | PASS |
| `python3 scripts/repository_safety.py --self-test` | PASS |
| `python3 scripts/repository_safety.py --worktree` | PASS — no findings |
| Godot headless (via core godot tests) | PASS in core suite |
| Live OpenAI | NOT TESTED / OFFLINE ONLY unless owner configures a key and sends |
| Owner mouse acceptance | UNVERIFIED (framebuffer / env-zoom harness is not mouse-driven Save→Open) |

## Evidence

`docs/handoffs/evidence/r4-reference/` (original R4):

- `laptop-1366x768-reference-beside.png` — reference beside Goomba reconstruction
- `restored-980x640-reference-stacked.png` — restored window compare
- `compact-800x500-reference-stacked.png` — compact controls + compare columns
- `laptop-1366x768-discover-open-reference.png` — Discover **Open reference**
- `providers-ai-optional.png` — AI providers, ask-before-send, no automatic call
- `original-brown-square.png` — original synthetic fixture (not commercial art)

`docs/handoffs/evidence/r41-zoom-restore/` (R4.1 correction):

- `original-synthetic-ref-64.png` — original 64×64 synthetic reference (not commercial art)
- `restored-reference-zoom8.png` / `restored-reference-zoom16.png` — saved creation reopen + reference zoom change
- `independent-zooms-ref24-recon4.png` — independent pane zooms
- `compact-800x500-reference-zoom12.png` — compact layout
- `orphan-rewire-restored.png` — ephemeral candidate id recovered onto saved sprite
- `laptop-1366x768-recon-zoom16.png` — wide layout, reconstruction zoom independent

## Acceptance checklist (owner)

- [ ] Open UnMaker, select a reconstructed asset, open Reference compare
- [ ] Import a local reference; see it beside the reconstruction
- [ ] Drag **Reference zoom** and confirm the reference image changes size (scroll when large)
- [ ] Drag **Reconstruction zoom** independently
- [ ] Save the project; quit; Home → **Open a Saved Creation**; confirm both images return
- [ ] Confirm AI stays idle without credential / without Send
- [ ] Optional: configure a local credential and Send once; review suggestion only

## Out of scope

R5 animation editing, R6 behavior authoring, R7 packaging, Mario/Luigi reconstruction without verified evidence, merge to `main`.
