# Risk and migration register

R0. No code migration has been done.

| Risk | Evidence | Handling |
| --- | --- | --- |
| Approve does not create a sprite | `approve_candidate` only sets status | R2 writes `SpriteDef` before reporting success. |
| Export collapses animation | `embed_form` takes `frames.iter().take(1)`; `import_package` reuses one frame id | Fix before any Goomba demo is called an animation. |
| Session dies on exit | `ExtractUi` is not a `Project` field. `app.rs` has no session save | Persist through the project or the baked sprite. Do not put ROM bytes in the JSON. |
| Palette and timing overclaim | Coverage palette is `"unverified"`. Export ticks are the constant 8 | Labels stay unverified until a trace or a user correction. |
| False SMB1 names | `propose_session` copies `smb1_documented` only on payload hash match. Test `unknown_hash_does_not_receive_smb1_names` | Keep that test. |
| Custom reference art called a cartridge extract | `reference.rs` `RefClass::Custom` and `attribution` | KEEP the classifier. R4 must not drop it. |
| AI runs without consent | `analyze_reference` checks `ai_extract` and `consent_send`. Empty key returns invalid credentials | No change in R2. Goomba must not call it. |
| `app.rs` size | 6,133 lines | New reconstruction UI goes in its own module. |
| Maker regression | Play, blueprint, and terrain tests | Do not edit those modules for R1–R2. |
| Old projects | Serde defaults on `Project` | New fields need defaults. A missing reconstruction opens as an empty sprite list, which is today's behavior. |
| Godot missing | `toolchain.rs` `discover` looks for 4.7.2. Three tests panic if it is absent | Report as a missing tool. Do not treat it as a gameplay failure. Do not download Godot for R0. |
| Strip clipped at 1366×768 | UnMaker strip is one horizontal row in `studio.rs` | R1 layout must wrap or scroll. Not a theme redesign. |
| Secrets and ROMs | Package rejects `sk-`. Project must not store the ROM | Tests stay. Do not commit `smb1.nes` or downloaded sheets. |
| Alpha library | New alpha id must not delete older directories or the key | Recipe writes stay in the current alpha dir. |

Migration order: document (R0) → persist and preview one enemy (R2) → issues (R3) → references and AI (R4) → editor (R5) → blueprint attach (R6) → Maker handoff (R7). Delete nothing in R0.
