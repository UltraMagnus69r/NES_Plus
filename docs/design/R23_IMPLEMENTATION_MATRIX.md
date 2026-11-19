# R2.3 implementation matrix

Gates: Implemented, Functionally tested, Visually tested, Blocked, Future milestone.

Functionally tested means a Rust test or a driven UI action. Visually tested means a screenshot of this build, not a concept image. Owner acceptance is separate and is not marked here.

| Item | Gate | Evidence |
| --- | --- | --- |
| Design tokens in `studio.rs` | Implemented. Functionally tested. | `studio::tests` for shared red, warmer Maker surface, gallery columns, Game Style, part groups |
| Sans-serif UI face | Implemented. | `install_theme` keeps egui's proportional face. Bundled serif is not inserted |
| Shared header and workspace chips | Implemented. | `unmaker_top_bar`. Visual check depends on a captured frame |
| Discover wrapping gallery | Implemented. Functionally tested for column counts. | `gallery_columns` at 1146, 846, 760, and 400. Scroll id `discover-gallery` |
| Discover search and filters | Implemented. | `gallery_query` and existing `Group` filters. Not a new recognizer |
| Discover inspector | Implemented. | Shown at content width >= 900. Hidden below that |
| Goomba Review states, play, one save | Implemented. Existing tests still cover the sheet. | `character_sheet` expectations. UI buttons call `publish_selected_goomba` |
| Provisional palette warning | Implemented. | Chip in `draw_assembly`. Colors are still the review palette |
| Other UnMaker stages share type and buttons | Implemented. | Home, Import, Investigate, Create, Save titles |
| Maker floating parts tray and tools | Implemented. | `maker_overlays`. Disabled tools do not call a fake action |
| Placement grid and ghost | Implemented. | `paint_grid` still uses `pointer_cell` and 48px cells |
| Game Style chooser | Implemented. Functionally tested. | `choose_game_style` rejects 16-bit, Modern Pixel, and Retro Palette |
| Test Play | Implemented. | Same `toggle_play` as before |
| Responsive collapse | Implemented in code. | Inspector side panel only when screen width >= 1100. Narrow path uses a window |
| Mouse tour of SMB1 Goomba | Blocked on this machine. | `investigation_exposes_the_goomba_to_discover_without_a_launch_hook` panics: `/home/g/Desktop/Roms/smb1.nes` is not here. No ROM was substituted |
| Owner visual acceptance | Accepted by the owner on November 18, 2026. | Acceptance is the owner's decision. It does not convert framebuffer screenshots or env tours into a mouse pass. The accepted chrome is the later R6.1 visual label |
| 16-bit, Modern Pixel, Retro Palette rendering | Future milestone. | Shown disabled. Not R2.3 |
| Mario, Luigi, Koopa reconstruction | Future milestone. | Cards stay unresolved. Not R2.3 |
| Moving a placed part, course zoom, pan tool, snap toggle | Future milestone. | Disabled with reasons |
| R3 issue decisions | Authorized. First slice is in memory and unit-tested. It does not save decisions and it does not change the Investigate screen | `issues::tests`, six passed in this session. `docs/roadmaps/R3_FIRST_SLICE.md` |
| Historical Phase 10 | Not started. | Stays suspended |

Visual screenshot files, when captured from this build, are listed in the session handoff. Concept images in `docs/design/references/` are not test evidence.
