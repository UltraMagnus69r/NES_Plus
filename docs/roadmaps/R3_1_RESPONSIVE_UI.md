# R3.1 / R3.1.1 — Responsive UI and resolution independence

Status: **R3.1 implemented; R3.1.1 correction implemented** on branch `cursor/r31-responsive-ui-9ff8`. **Not owner-accepted.**

R3 (intelligent issue resolution) was owner-accepted on **2026-11-24**. R3.1 is a supplemental usability milestone between R3 and R4. Owner visual acceptance of R3.1 failed (clipping, overflow, weak reflow). **R3.1.1** is the corrective implementation. R4 remains unauthorized.

## Rationale

Nesticles looked polished near 1920×1080 but did not adapt cleanly to a short laptop client (historically ≈1366×694 after KDE chrome). Fixed nav widths, Discover inspector breakpoints, Maker tool strip width, and play-surface caps assumed a large desktop. The correction is **adaptive density and layout**, not a global `window_width / 1920` shrink.

## R3.1.1 root causes (production path)

Diagnosed on the Fedora laptop against the R3.1 tip before correction:

1. **Density ignored height.** Layout helpers keyed off width-only `mode`. A 1366×694 maximized client stayed **Standard** desktop chrome (wide nav, tall hero, side inspector thresholds) and starved Discover/Maker of usable content.
2. **Gallery card width clamped above the fit.** `gallery_card_width` used a fixed minimum (200pt) that could exceed the row budget when columns were computed independently — cards overflowed or clipped.
3. **Inspector split did not prove gallery + inspector ≤ content.** Side inspector could reserve width that forced the gallery offscreen.
4. **Category filters used a single horizontal `ScrollArea` without wrap.** Filters beyond the right edge looked “missing” rather than wrapped or obviously scrollable.
5. **Shell chrome fixed at desktop sizes.** Brand + workspace chips + actions overflowed restored ~800×500 windows (Maker chip truncated).
6. **Maker tool capsules used a fixed 161pt width** that overflowed Compact/Medium tool rails.
7. **Screenshot wait was too short for KDE/Wayland** to apply `InnerSize` / maximize before capture (historical evidence claimed 1366 while WM ignored the first resize).

## R3.1.1 corrections

Central policy remains `crates/nesticles/src/responsive.rs` → `ResponsiveLayout` / `LayoutMode`.

New / tightened helpers:

- `density()` — short clients (`height < 740`) densify one step (`Expanded→Standard→Medium→Compact`) so chrome matches usable height.
- `gallery_fit(width)` — columns + card width that never overflow the measured gallery width; Medium/Standard column cap 4; Expanded 5.
- `discover_split_widths(content)` — returns `Some((gallery, inspector))` only when both fit; otherwise drawer/collapse.
- Shell helpers: `shell_brand_width`, `shell_workspace_chip_width`, `shell_action_button_width`, `shell_shows_project_chip`.
- `filter_chip_height`, `gallery_card_height`, `discover_chrome_budget`, `maker_tool_button_width`.

Production call sites:

- UnMaker shell / nav / hero / Discover / Investigate: `unmaker/view.rs` (wrap filters, `gallery_fit`, split/drawer inspector, density-based chrome).
- Maker tools / parts / drawer: `app.rs` (density heights; tool buttons use `available_width`).
- Shared gallery helpers + clamp: `studio.rs`.
- Shot harness: longer wait + re-assert `InnerSize` so captures match requested geometry under Wayland.

## Breakpoint policy (logical points)

| Available UI width | Mode |
| --- | --- |
| &lt; 800 | Compact |
| 800–1199 | Medium |
| 1200–1699 | Standard |
| ≥ 1700 | Expanded |

Height densifies chrome independently of width. Modes still use **logical egui points**, not raw monitor pixels.

## Screen-by-screen (R3.1.1)

### UnMaker

- **Shell:** Compact titles/chip widths on short Medium clients; Project chip only when width ≥ 1180 and density is not Compact.
- **Discover:** Wrapping category filters (horizontal scroll fallback on extremely narrow UIs); gallery columns from `gallery_fit`; side inspector only via `discover_split_widths`; otherwise selected-asset drawer.
- **Investigate:** Meta collapses on Compact/Medium density or height &lt; 720 so Accept/Skip/Reject remain reachable. Semantics unchanged.
- **Review:** Still inside stage scroll; nearest-neighbor policy unchanged.

### Maker

- Tools sidebar width from `maker_tools_width()`; capsules fit the rail.
- Parts strip shortens under Compact/Medium density.
- Course canvas still uses `EDITOR_CELL_PX` / `pointer_cell` — **window resize does not rescale game coordinates**.
- Test Play remains on the Maker dock / shell.

## DPI and window geometry

1. Display DPI — egui `pixels_per_point` / KDE scaling.
2. UI density — breakpoints, wrapping, collapsing, scrolling.
3. Game-world zoom — Maker canvas / sprite preview.

`MIN_INNER` = 800×500. `clamped_inner_size` still prevents a 4K-saved size from overflowing a laptop monitor. Decorations, maximize/restore, and `persist_window` remain. Fullscreen is not forced.

## Automated tests

`responsive::tests` (among others):

- Breakpoint boundaries; usable width after panels
- Laptop densifies Standard→Medium; short Medium→Compact shell
- Gallery never overflows row; Discover split stays inside content
- Inspector collapse; nav allocation; shell chrome budget
- Maker tools fit laptop; Investigate vertical budget
- Canvas cell mapping independent of window size
- DPI-equivalent logical layouts; viewport transition recomputes

`studio::tests` updated for Medium/Standard column caps and no-overflow card fit.

R3 `nesticles-core::issues` remains the suggestion/Accept/Skip/Reject persistence gate.

## Visual evidence (R3.1.1)

Framebuffer `NESTICLES_SHOT` captures under `docs/handoffs/evidence/r311-responsive/` (rendering evidence, not a mouse pass):

| File | Measured size | Notes |
| --- | --- | --- |
| `restored-discover-1366x694.png` | 1366×694 | 4-column gallery; filters reachable; inspector collapsed |
| `restored-discover-800x500.png` | 800×500 | Compact shell + icon nav; filters wrap |
| `restored-discover-980x600.png` | 980×600 | Restored medium window |
| `restored-maker-1366x694.png` | 1366×694 | Tools rail + Test Play |
| `restored-maker-980x600.png` | 980×600 | Maker restored |
| `restored-home-1366x694.png` | 1366×694 | UnMaker Home |
| `restored-investigate-1366x694.png` | 1366×694 | Investigate chrome |
| `restored-review-1366x694.png` | 1366×694 | Review issue panel |
| `maximized-discover.png` / `maximized-maker.png` / `maximized-home.png` | 1920×1006 | Current Fedora panel maximized client (1920×1080 panel @ scale 1) |

1920×1080 / 2560×1440 / 3840×2160 layout math covered by Expanded/Standard helpers. Physical 4K GUI **UNVERIFIED** on this machine (no 4K panel). SMB1 authentic gallery art **BLOCKED** (owner ROM absent); Discover evidence used the synthetic fixture ROM for layout only.

## Known limitations

- Compact nav uses icon-only capsules; labels move to hover text.
- Discover inspector as a drawer on compact/laptop density is scrollable, not a second workspace.
- No pixel-perfect screenshot golden tests (intentionally).
- Owner mouse acceptance on the laptop display is still required.
- Maker Fill / Inspect / Undo / Redo may sit below the tools scroll fold on short clients (reachable by scroll).
- R4 not started.

## Acceptance distinction

- **R3**: owner-accepted (functionality).
- **R3.1**: implemented; visual acceptance failed.
- **R3.1.1**: correction implemented + automated tests + framebuffer GUI evidence; **owner acceptance pending**.
- **R4**: not authorized.
