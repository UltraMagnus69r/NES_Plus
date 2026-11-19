# Nesticles Design Language 2.0

Unified UI system for UnMaker Lab and Maker Engine. This is the R2.3 behavior and appearance contract. It records the owner's assignment and the Design Language board supplied with that assignment. It is not a new design exploration.

When sources disagree, use this order:

1. Existing functional correctness and data integrity.
2. This document.
3. Usable responsive behavior.
4. Illustrated mockup details.

Mockups set composition, hierarchy, spacing, styling, and the overall product feeling. They do not mean that every depicted asset, course, or reconstruction already exists. Never invent functionality or source artwork to match a picture.

## 1. Product

Nesticles is one native desktop application with two workspaces.

- UnMaker Lab investigates, discovers, and reconstructs. Identity: a yellow speckled egg. Personality: elegant, inviting, calm, intelligent, visual, approachable. Surfaces are cool white, pale blue, and restrained lavender, with yellow used for selection.
- Maker Engine builds and plays. Identity: a small original geometric castle. Personality: playful, colorful, tactile, creative, immediate. Surfaces are warm neutrals with the same action colors.

Both workspaces share one token set, one type ramp, and one control language. The native operating-system title bar stays. The application does not draw minimize, maximize, or close.

## 2. Tokens

Colors from the Design Language board:

| Token | Value | Use |
| --- | --- | --- |
| Nesticles Red | `#E53935` | Primary destructive or commit actions, including Exit and Approve |
| Apple Blue | `#0A84FF` | Selection, links, active navigation |
| Game Green | `#34C759` | Test Play and a successful reconstructed state |
| Accent Yellow | `#FFCC00` | UnMaker selection and the egg |
| Accent Purple | `#AF52DE` | Castle accent |
| Surface | `#FFFFFF` | Cards and buttons at rest |
| Surface Alt | `#F6F8FF` | UnMaker page background |
| Panel | `#EEF2F9` | Quiet fills |
| Border | `#D6E3F0` | Hairlines |
| Maker surface | `#FFF8F2` | Maker panels |
| Maker mist | `#FFF1E4` | Maker hover and selection |
| Ink | `#1C1C1E` | Text |
| Caption | `#46505C` | Secondary text |

Spacing scale: 4, 8, 12, 16, 24, 32. Corner radius scale: 4, 8, 12, 16, 24, 32. Controls use 10–16.

Type, implemented with egui's built-in proportional sans. Proprietary faces and the bundled serif are not the UI face.

| Role | Size |
| --- | --- |
| Title | 28 |
| Header | 20–24 |
| Section | 16–18 |
| Body | 13–16 |
| Caption | 11–12 |

States: Normal, Hover, Pressed, Disabled. Status colors: Reconstructed green, Needs Review blue, In Progress yellow, Unresolved gray. Disabled controls say why.

## 3. Shared components

The implementation uses these roles. Names in code may differ when they match the module they live in.

- Asset card
- Status chip
- Segmented state control
- Preview surface
- Floating tool tray
- Context inspector
- Primary action
- Notice
- Game Style chooser
- Responsive gallery

Presentation components read the canonical sprite and animation. They do not keep a second copy.

## 4. UnMaker Lab

Header, left to right: Nesticles wordmark and egg, UnMaker Lab chip, Maker Engine chip, project or cartridge label, Save, Menu, Exit. Exit remains because it is existing behavior. The window buttons still work.

Left navigation: Home, Import, Investigate, Discover, Review, Create, Save. The selected stage is unmistakable. Technical diagnostics stay behind Technical details. The build disclaimer is a collapsed About this build section.

### Discover

Discover is a vertically scrolling wrapping gallery. It is not one horizontal row.

- Consistent cards.
- Clear selection.
- Mouse-wheel scrolling and a visible scrollbar.
- Category filters for every existing group, including Ignored.
- A name search over the cards already produced by the investigation.
- No horizontal overflow.
- Scroll position and selection survive a trip to Review and back. egui keeps the scroll id `discover-gallery`. Selection is `StudioState.selected`.

At a wide client, target three or four cards per row, never more than four. Narrower windows reduce the column count. At or above 900 points of content width, a side inspector shows the selected card, its real preview when one exists, and Open in Review. Below that width the inspector is omitted and the gallery keeps the width.

Cards show a real reconstructed thumbnail only for the assembled Goomba. Other cards show a source tile or an honest empty mark. The Goomba is one card. Walk and Defeat are states inside it.

### Review

Review owns the character workspace. For the assembled Goomba it shows the name, reconstructed status, the default form, Walk, Defeat / Stomped, a large nearest-neighbor preview, Play, Pause, frame count, zoom, a provisional-palette warning, collapsible source evidence, and exactly one Approve & Save Goomba action. Playback uses the existing assembler and pose clock. Guided assembly is not the default path for that Goomba.

Other candidates keep the existing issue actions. Unavailable actions stay disabled and explain why. Mario, Luigi, and Koopa are not given invented graphics.

### Other stages

Home, Import, Investigate, Create, and Save use the same type, cards, and buttons. Incomplete work stays labeled as incomplete. Create does not pretend a finished authoring flow exists. Save still writes the project file and does not store the ROM.

## 5. Maker Engine

The course is the main surface. Parts and tools float above it. They do not reserve a permanent top strip and left rail.

The parts tray lists terrain brushes and picture assets that are already in the project. Categories are All, Terrain, Players, Enemies, Items, and Other. An empty group says so. The tray does not invent a sample level.

Editing controls that exist: Select, Place, Erase, Line, Vertical, Fill (the existing rectangle tool), Inspect, Undo, Redo. Move, Zoom, Pan, Snap, and a grid hide-toggle are visible and disabled, each with the reason. The grid is always drawn on the real cell size. A placement ghost shows the hovered cell when a part or brush is selected. Coordinate math is unchanged.

The inspector opens when a placed part is selected. On a wide window it is a right panel. On a narrow window it is a collapsible window. Hide inspector leaves the selection. Clear selection removes it. Course settings stay in More.

Test Play calls the existing Rust Play runtime. It is the green button in the header and the Play control in the dock.

### Game Style

Classic 8-bit is the only selectable style. It does not convert resources. 16-bit, Modern Pixel, and Retro Palette are shown and cannot be selected. Choosing them does not change the course. The chooser is a floating chip. Closing it does not block editing.

## 6. Responsive targets

- Maximized 1366×768, about 1366×694 of client area.
- Restored window about 980×600.
- Larger displays when available.

UnMaker reduces gallery columns, keeps vertical scrolling, and drops the side inspector when narrow. Maker collapses the inspector to a window, keeps Test Play in the header, and keeps the tool tray on the canvas. The window does not have to be fullscreen.

## 7. Out of scope

R2.3 does not add reconstruction algorithms, image recognition, gameplay, graphics engines, style conversion, a Godot overhaul, Mario or Koopa reconstruction, or a broader CHR investigation. It does not upload a ROM or call an external model by itself. It does not ship mockup artwork as game art.
