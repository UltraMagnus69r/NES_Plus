# Nesticles

Nesticles is a local workshop for turning a cartridge image you already have into pieces of an original game, then playing that game on the desktop or in a browser.

You open a ROM, page it as tiles, tag the pieces yourself, keep sounds, and paint courses. A small overworld links those courses. Clearing one unlocks the paths that leave it. The desktop window is the editor and the player. Export writes a folder you can open in Chrome, Firefox, or Edge, including on Android.

The demo board is an original game called Seed Spirit. Pip is an original seed character. The sounds are synthesized here. Nothing in the repository is a commercial ROM, sprite sheet, or soundtrack.

## Flow

1. **Cartridge.** Open a file, or load the demo image from the top bar. Nesticles reads NES / iNES / NES 2.0 (the 512-byte trainer flag included), Game Boy, Game Boy Color, Super NES LoROM and HiROM (a 512-byte copier header included), and Game Boy Advance. A file it does not recognize still opens as a tile pager. Files larger than 64 MB are refused. Encrypted Switch, 3DS, and Wii U images are not decrypted.
2. **Tiles.** The pager shows uncompressed tiles with nearest-neighbor scaling. Codecs are NES 2bpp (planes separated), Game Boy 2bpp (planes interleaved per row), Super NES 4bpp, Game Boy Advance 4bpp (low nibble first), and 1bpp. A deduped scan suggests empty, sprite-like, scenery-like, text-like, or noise. You tag the tile: player, enemy, terrain, background, foreground, item, hazard, or goal. Cartridges do not label the player versus an enemy, so the tag has to be yours. Add as part stores a PNG part on the current style. Export PNG writes that picture without placing it.
3. **Audio.** Preview NES DPCM (least-significant bit first, start level 64, steps of 2, clamped to 0–126), Game Boy 16-byte wave tables, and Super NES BRR. Add a clip as a sound or as music on the same style. PCM WAV import is built in. Ogg and MP3 import when the decoder can open the file.
4. **Notes.** ASCII strings, plus a linear 6502 listing for NES program bytes. The listing does not follow banks and it will read data as code. Scripts are custom per game, so they are not extracted. Physics is not extracted either. Play uses an original engine: gravity, acceleration, friction, a variable jump, coyote time, a jump buffer, AABB tile collision, patrol enemies that turn at walls and ledges, a stomp, hazard and fall respawns, and a goal.
5. **Styles, courses, overworld.** Each cartridge is a style of parts. Opening a second game adds a style and leaves the first one in the tray. A course has a home style. The tray can show that style or every style, and you drag parts onto the grid. Parts from more than one style can share a course. The overworld links courses. Play from the overworld. Clearing a course unlocks the paths that leave it.
6. **Export.** Writes a folder: `index.html`, `project.json`, `README.txt`, `manifest.webmanifest`, `icon.png`, and, when the wasm player was built into the desktop app, the wasm module and its JavaScript glue. The page inlines the module so opening the HTML file is enough to play. The icon is the favicon, the Apple touch icon, and the web-manifest icon. Adding it to an Android home screen needs https.

## Two API keys

API keys is one dialog with two slots.

- **Workshop assistant.** OpenAI, Anthropic, or a custom OpenAI-compatible base URL. The model id is an editable field. Suggested starting points are `gpt-4.1-mini` and `claude-sonnet-4-5`. The assistant can create and rename courses, set a course's home style, paint rectangles and cells from any style, set the player start, rename or retarget assets, add, move, and link overworld nodes, set the start node, and audit whether a course has ground, a start, and a goal. It cannot call the image model. If you ask it to draw, it points you at the Image maker.
- **Image maker.** A separate key, used only when you press Generate. OpenAI images (`gpt-image-1` or `dall-e-3`) or a custom OpenAI-compatible `/images/generations` endpoint. The picture is downscaled and quantized to a 16, 32, or 48 pixel sprite and added to the library as the kind you picked. Prompts that name commercial game characters are refused.

Rust calls the providers with reqwest. There is no browser CORS proxy. Keys are stored in the OS config directory (`keys.json`, mode `0600` where the OS allows it). They are not written into the project file, not written into an export, and redacted from error text.

## Play on Linux, Windows, and Android

- **Linux and Windows.** `cargo run -p nesticles` (or the release binary) is the editor and the player. This repository was built and tested on Linux. The same crate is the Windows build. This environment did not cross-compile a Windows executable; a mingw toolchain was not installed. On Windows, install Rust and run `cargo build -p nesticles --release`.
- **Android.** Use the exported browser folder. Open `index.html` in Chrome, Firefox, or Edge. On-screen Left, Right, and Jump buttons are on the page, and the keyboard works the same way (arrows or A/D, jump with Z, W, Up, or Space). File open is enough to play. Add to Home Screen needs the folder served over https. Nesticles does not produce an Android package.

Rebuild the browser player with `scripts/build-player.sh`, then rebuild the desktop app so export embeds the fresh wasm. If that toolchain cannot be installed, the desktop player and project JSON export still work, and the export note says what failed.

## Layout

```
crates/nesticles-core     project JSON, codecs, audio, disassembly, physics, tools, prompt guard, demo ROM
crates/nesticles          eframe editor, in-app play, assistant, keys, image maker, export
crates/nesticles-player   wasm32 cdylib. The same physics step as the desktop player
```

`cargo test` and `cargo build` cover the workspace. The player target is `wasm32-unknown-unknown`.

## When the graphics are compressed

Nesticles pages uncompressed tiles. It does not replace a tool that already knows a game's compression, palettes, or script bytecode.

- [Dazzlie](https://github.com/obskyr/dazzlie) converts Game Boy and NES tiles to and from PNG.
- [nesrip](https://github.com/FitzRoyX/nesrip) builds NES and Super NES tilesheets. It needs its graphics database.
- [RetroTool](https://github.com/danielburgess/RetroTool) covers Super NES codecs and scripts, and it can emit Godot or Tiled data.
- [TileBravo](https://github.com/taihendarou/tilebravo) is a tile editor.
- [gbgraphics](https://github.com/drpaneas/gbgraphics) finds Game Boy tiles from a screenshot.
- [Mesen](https://github.com/SourMesen/Mesen2) and [SameBoy](https://github.com/LIJI32/SameBoy) are debuggers and tile viewers for when a game builds its patterns in RAM.
- [Tiled](https://github.com/mapeditor/tiled) and [Godot](https://godotengine.org/) are the next stop if you later want a native build for a mobile store.

## Font

The window titles use Liberation Serif, bundled under the SIL Open Font License. See `crates/nesticles/assets/LICENSE-Liberation.txt`.
