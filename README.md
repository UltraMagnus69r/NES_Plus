# Emberleaf

Emberleaf is a browser workshop for turning tiles you have the right to use into a small original platform game. You open a classic unencrypted ROM locally, pull out tiles and uncompressed audio one piece at a time, tag what they are, then build courses and an overworld. The playable game is Emberleaf’s own physics, not code taken from a console.

Pip, the seed-spirit, the sample courses, the sound cues, and the demo cartridge are original. The demo cartridge is an iNES file generated in the app so you can try extraction without bringing a dump.

## Pipeline

1. **Source.** Drop a ROM, a PNG, or an audio file, or load the demo cartridge. Files stay in the browser tab and are not uploaded. Anything over 64 MB is refused.
2. **Extract.** Page the file as tiles (NES 2bpp, Game Boy 2bpp, Super NES 4bpp, Game Boy Advance 4bpp, or 1bpp). A candidate scan skips blank tiles and guesses sprite, scenery, text, or noise. You multi-select, name a part, and choose a kind. Audio can pull NES DPCM, Game Boy wave tables, and Super NES BRR where those bytes are stored uncompressed. The code tab lists ASCII strings and shows 6502 or LR35902 machine code. It does not recover scripts or physics.
3. **Library.** Rename parts, set solid and behavior (patrol, collect, hurt, goal, player, decoration).
4. **Levels.** Paint courses. Terrain is solid. Backgrounds are not. Actors spawn from enemy, item, hazard, and goal parts.
5. **Overworld.** Drag nodes, link them, pick a start. Clearing a course unlocks its neighbors. Clearing every linked course wins.
6. **Play.** Keyboard (arrows or WASD, Space or Z) and on-screen controls for coarse pointers.
7. **Export.** A standalone HTML file, a zip, or the project JSON. The project save is the library and levels, never the raw ROM.

## Run

```bash
npm install
npm test
npm run dev
npm run build
```

## Export platforms

The HTML file opens directly in Chrome, Firefox, or Edge on Linux, Windows, and Android. The zip also contains `manifest.webmanifest` and a short readme. Add to Home Screen needs that folder hosted over https. Opening the HTML file does not.

The runtime is one plain JavaScript file, `src/runtime/player.js`, used by the workshop and copied into the export. There is no emulator and no Nintendo game code in the download.

## Legal

Bring dumps you may use. Emberleaf does not include Nintendo sprites, music, ROMs, or code. It does not decrypt Switch, 3DS, Wii U, or any other encrypted system. Compressed or banked graphics, custom scripts, and console physics are outside what a universal browser scan can honestly recover.

## Tools to use alongside Emberleaf

Tiles in a ROM are not labeled. When graphics are compressed, banked, or only make sense next to a screenshot, use a tool that knows that system:

- **Dazzlie** — Game Boy and NES tiles to and from PNG.
- **nesrip** — NES and Super NES tilesheets. It needs its graphics database and understands some compression layouts.
- **RetroTool / SFCRetroTools** — Super NES codecs and scripts, with Godot and Tiled emitters. https://github.com/danielburgess/RetroTool
- **TileBravo** — a tile editor in the YY-CHR tradition.
- **gbgraphics** — find Game Boy tiles from a screenshot.
- **Mesen** — NES, Super NES, and Game Boy tile and sprite viewers.
- **SameBoy** — a Game Boy debugger.
- **Tiled** and **Godot** — the path if you later want a native project, an APK, or a desktop binary.

Emberleaf’s choice is a browser runtime so one file plays on Linux, Windows, and Android without an emulator and without shipping someone else’s game code.
