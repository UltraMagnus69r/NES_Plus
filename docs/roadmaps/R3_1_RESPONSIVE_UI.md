# R3.1 — Responsive UI and resolution independence

Status: **implemented on branch `cursor/r31-responsive-ui-9ff8`**. Not owner-accepted.

R3 (intelligent issue resolution) was owner-accepted on **2026-11-24** in the session that authorized this milestone. R3.1 is a supplemental usability milestone between R3 and R4. R4 remains unauthorized.

## Rationale

Nesticles looked polished near 1920×1080 but did not adapt cleanly to the Fedora laptop’s **1366×768** display (usable maximized client area historically ≈1366×694 after KDE chrome). Fixed nav widths, Discover inspector breakpoints, Maker tool strip width, and play-surface caps assumed a large desktop. The correction is **adaptive density and layout**, not a global `window_width / 1920` shrink.

## Existing layout defects (inventory)

| Area | Defect |
| --- | --- |
| `studio::MIN_INNER` | 960×540 blocked restored compact windows |
| UnMaker nav | Fixed 236 pt rail on every width |
| Discover | Inspector break at 1080; gallery capped without mode awareness |
| Stage hero | Fixed 122 pt height stole vertical space on laptops |
| Investigate | Large checklist/stats stacked above R3 Accept/Skip/Reject |
| Maker tools | Fixed 212 pt sidebar |
| Play surface | Hard clamp to 1920×1080 |
| Window restore | Origin clamped; oversized 4K inner size not clamped to monitor |

## Responsive architecture

Central policy: `crates/nesticles/src/responsive.rs` → `ResponsiveLayout` / `LayoutMode`.

Derived each frame from the egui viewport (`inner_rect` when available):

- width / height in **logical points**
- `pixels_per_point` (recorded; layout modes do **not** key off raw pixels)
- helpers: nav width, hero height, gallery columns, inspector visibility, Maker tools width, page margins, content-after-panels

Call sites consume one layout snapshot instead of scattering ad-hoc width checks:

- UnMaker: `unmaker/view.rs` (`unmaker_studio`, Discover, Investigate, stage hero/buttons)
- Maker: `app.rs` (`maker_shell`, play surface fit)
- Shared tokens: `studio.rs` (`gallery_columns`, `discover_shows_inspector`, `clamped_inner_size`, lowered `MIN_INNER`)

## Breakpoint policy (logical points)

| Available UI width | Mode |
| --- | --- |
| &lt; 800 | Compact — icon rail, drawer inspector, collapsed Investigate meta |
| 800–1199 | Medium — narrower nav, limited gallery columns |
| 1200–1699 | Standard — full workspace when space permits |
| ≥ 1700 | Expanded — more gallery columns, comfortable panels |

Height is considered separately (e.g. Investigate collapses tall meta when height &lt; 720).

## Screen-by-screen changes

### UnMaker

- **Home / Import / Create / Save**: adaptive stage hero + page margins; vertical scroll retained.
- **Discover**: column count from available gallery width and mode; side inspector only when `show_discover_inspector`; otherwise selected-asset drawer with scroll.
- **Review**: remains inside the stage scroll area (Approve & Save reachable by scroll). Nearest-neighbor sprite policy unchanged.
- **Investigate**: Stages checklist becomes a collapsing header on compact/short viewports; candidate stats compress to chips; **R3 suggestion fields and Accept/Skip/Reject semantics unchanged**.

### Maker

- Parts strip height shortens in Compact; course summary hides below ~1030 width.
- Tools sidebar width from `maker_tools_width()`; tools remain in a vertical scroll; Game Style stays under the scroll.
- Course canvas still uses `EDITOR_CELL_PX` / `pointer_cell` — **window resize does not rescale game coordinates**.
- Test Play remains on the Maker dock (unchanged control surface).

## DPI handling

Three separate concepts:

1. **Display DPI** — egui `pixels_per_point` / KDE scaling.
2. **UI density** — breakpoints, wrapping, collapsing, scrolling.
3. **Game-world zoom** — Maker canvas and sprite preview systems.

Modes are selected in points, so 100% / 125% / 150% scaling with the same logical size share a mode. Tile picking ignores desktop DPI.

## Window geometry

- `MIN_INNER` = 800×500.
- `clamped_inner_size` + startup path in `place_window_if_needed` prevent a 4K-saved size from overflowing a laptop monitor.
- Decorations, maximize/restore, and `persist_window` remain.

## Test coverage

`responsive::tests` (binary crate):

- Breakpoint boundaries
- Usable width after panels
- Gallery wrapping / inspector collapse
- Maker tool fit on laptop client
- Vertical reachability budget for Investigate
- Canvas cell mapping independent of window size
- DPI-equivalent logical layouts
- Compact↔expanded policy identity

`studio::tests` updated for new gallery/inspector helpers and clamp.

R3 issue decision tests in `nesticles-core::issues` remain the regression gate for suggestion/Accept/Skip/Reject persistence.

## Visual acceptance matrix

| Target | How verified in this session |
| --- | --- |
| 1366×768 | Primary policy target in unit tests; GUI launch optional |
| 1920×1080 / 2560×1440 / 3840×2160 | Layout math via Expanded/Standard helpers |
| 980×600 / 800×600 | Compact/Medium unit coverage |
| Physical laptop GUI | Maximized-client framebuffer at 1366×694 captured; mouse pass pending owner |

Framebuffer `NESTICLES_SHOT` captures are **rendering evidence**, not a mouse pass.
Available capture: `docs/handoffs/evidence/r31-responsive/maximized-laptop-client-1366x694.png`
(`NESTICLES_INNER` did not change capture size under WM timing in this session — do not invent multi-resolution shot claims.)

## Known limitations

- Compact nav uses icon-only capsules; labels move to hover text.
- Discover inspector as a bottom drawer on compact is scrollable but not a separate workspace.
- No pixel-perfect screenshot golden tests (intentionally).
- Owner mouse acceptance on the laptop display is still required.
- R4 not started.

## Acceptance distinction

- **R3**: owner-accepted (functionality). Mouse-driven Investigate actions were **not** independently re-verified solely because of that acceptance.
- **R3.1**: implemented + automated tests; **owner acceptance pending**.
