# R2.3 view inventory

Source of the screens before and after the visual pass. Behavior that already worked stays in the same functions.

## UnMaker Lab

Shared shell: `crates/nesticles/src/unmaker/view.rs` (`unmaker_top_bar`, `unmaker_studio`) and `crates/nesticles/src/studio.rs` tokens. Stage state is `StudioState` in `unmaker/model.rs`. The extraction session stays in `ExtractUi`. Saving the project does not store the ROM.

| Screen | Module | Existing behavior | Handlers | Persistent state | R2.3 presentation | Preserved | Incomplete |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Header | `view.rs` `unmaker_top_bar` | Save, Menu, Exit, workspace flip | `save_project_quick`, `app_menu`, `request_exit`, `flip_workspace`, `toggle_play` on Maker | Project file, dirty flag, window geometry | Wordmark, egg and castle chips, project caption | Native title bar, F6 flip, Ctrl+S | Exit is still an in-app button as well as the window close |
| Home | `home_stage` | Disassemble, open, new, continue | Stage changes, `open_project_dialog` | Project path, ROM session, sprites | Title and cards | Continue rules in `continue_available` | No suggested projects |
| Import | `import_stage` | Open ROM, optional reference URL, start investigation | `choose_studio_rom`, `start_reference_fetch`, `start_investigation` | ROM bytes in memory only, reference URL | Same form, shared buttons | Reference safety in `openai.rs` | A blocked fetch is still a message, not a new recovery design |
| Investigate | `investigate_stage` | Stage log, cancel, counts, tools | `job_cancel`, stage change, `tool_row` | Job and session in memory | Shared title | Notes, providers, guide tools | No new investigation engine |
| Discover | `discover_stage` and gallery helpers | Cards from `gallery_items` | Filter, search, select, `open_candidate_in_review` | `StudioState.filter`, `gallery_query`, `selected`, scroll id `discover-gallery` | Wrapping vertical gallery, status chips, optional inspector | One Goomba card, real thumbnail only when assemble succeeds | Search only filters names already on the cards |
| Review | `review_stage`, `character_review`, `draw_assembly` | Goomba walk and defeat, one save, issue actions for everyone else | Pose, play, pause, zoom, `publish_selected_goomba`, ignore, skip, manual puzzle | `preview_pose`, `preview_playing`, project sprite after save | Segmented states, large nearest-neighbor well, provisional chip, collapsible evidence | Assembler, timing, `SpriteDef`, `AnimSet`, no default puzzle for a successful Goomba | Palette is still the review palette |
| Create | `create_stage` | Assemble and Animate on project sprites | `create_animate`, embedded animation UI | Project sprites and anim sets | Shared title | Saved Goomba plays here without the ROM | Not a new creation wizard |
| Save | `save_stage` | Project save, library, export, overworld | `save_project_quick`, `tool_row` | Project file, alpha library | Shared title | Package still keeps frames. First-frame readers still see the first frame | Export is not a finished character |
| Embedded tools | `embedded_tool` | Tiles, audio, library, export, notes, overworld, providers, behavior, guide | Existing workshop functions | Project and session | Back button and shared type | Each tool's old behavior | Not restyled pixel by pixel |

## Maker Engine

Shell: `app.rs` `maker_shell`, `maker_overlays`, `paint_grid`, `maker_inspector`, `game_style_chooser`. Play: `toggle_play` and `play_shell`.

| Screen | Module | Existing behavior | Handlers | Persistent state | R2.3 presentation | Preserved | Incomplete |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Course canvas | `paint_grid` | Place, erase, terrain strokes, select a cell | Pointer, `place_at`, `terrain_pointer` | Level cells in the project | Full central surface, stronger grid, hover ghost | Cell size and `pointer_cell` math | No sample level. Empty projects get New course |
| Parts tray | `maker_palette` | Brushes and assets | Select brush or asset, search | `maker_group`, `palette_query`, `terrain_brush`, `paint_asset` | Floating tray and category chips | Only real project parts | No style conversion of those parts |
| Tools | `maker_tool_column`, `apply_rail` | Select, paint, erase, line, vertical, rectangle, inspect, undo, redo | `apply_rail`, `undo_terrain`, `redo_terrain` | `rail`, undo stacks | Floating labeled tray. Move, Zoom, Pan, Snap, Grid-hide disabled with reasons | The enabled tools still call the old operations | Move of an existing part is not implemented |
| Inspector | `maker_inspector` | Selected cell, animation picker, blueprint link | Hide, clear, drawer | `placement_pick`, `inspector_open` | Side panel when wide, window when narrow | Cell data unchanged | Does not invent properties |
| Courses | `courses`, `add_course` | Rename, new course, play | More drawer | Level list | Same drawer | Sky of old courses stays. New courses use a light sky so the grid reads | Not a world map |
| Test Play | `toggle_play` | Rust Play | Header Test Play, dock Play, F5 | Play state, not a second project | Green Test Play | Same runtime | Godot preview is still the experimental menu item |
| Game Style | `game_style_chooser` | None before R2.3 | `choose_game_style` | UI state only. Not written into the project as a conversion | Floating chip. Classic 8-bit only | Current renderer unchanged | 16-bit, Modern Pixel, and Retro Palette are unavailable |

The older `unmaker_shell` in `app.rs` is not on the live path. `update` opens `unmaker_studio` for UnMaker and `maker_shell` for Maker.
