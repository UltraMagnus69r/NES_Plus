# Nesticles Alpha 1 — known limitations

## What this alpha is

A Linux x86_64 desktop editor. Rust Play is the gameplay test inside the app. You can import PNG art, page a supported cartridge for pictures, assign Blueprints, paint connected terrain, play, save, and reopen.

## Cartridge formats

| Format | Header detection | Tile paging in this alpha |
|---|---|---|
| NES / iNES / NES 2.0 | Yes | Uncompressed 2bpp. CHR RAM cartridges have no single graphics sheet. |
| Game Boy | Yes | Uncompressed 2bpp. Many pictures exist only after game code builds them. |
| Game Boy Color | Yes | Same as Game Boy. |
| Super NES LoROM / HiROM | Yes | Uncompressed 4bpp. Compressed art is not decoded. |
| Game Boy Advance | Yes | Uncompressed 4bpp. Compressed art is not decoded. |
| Switch, 3DS, Wii U | Not decrypted | Not supported. |
| Anything else | Reported as unknown | Opened only as a raw tile pager, and the status line says so. |

Nesticles does not import gameplay, physics, enemy AI, or level scripts from a cartridge.

## Godot

Godot 4.7.2 preview covers walk, run, skid, jump, hold-jump, falling, solid collision, Goomba movement and stomps, and goals. It does not yet simulate other enemies, question blocks, items, power-ups, hazards, or pipe transitions. Those still run in Rust Play when the Blueprint says so.

A Player Blueprint's picture and physics preset are written into the preview. The preview is not full parity with Rust Play.

## Player

Collision uses the production play hitbox. Small, Super, and Fire change height the same way a power-up does. There is no separate custom hitbox editor. Courses saved before this alpha have no Player Blueprint and keep the first character picture plus the project physics preset. Opening them does not invent a blueprint.

## Host libraries

The program is a normal Linux x86_64 executable. It needs glibc 2.39 or newer, GTK 3 (`libgtk-3.so.0`), and GLib (`libglib-2.0.so.0`) from the system. Fedora and current Bazzite images that already run GTK applications have these. OpenGL comes from the desktop's own Mesa driver. The package does not include the C library or the GPU driver.

If launch fails with `libgtk-3.so.0: cannot open shared object file`, GTK 3 is not installed for this user. Nesticles does not ask you to rebase the OS. Rust Play does not need Godot.

## Not in this package

Windows build. Flatpak. A system RPM. The Godot editor itself. Commercial ROMs. API keys. Your project files. Build caches.

AI assistant and image generation stay optional. The editor, save, and Rust Play work with no network and no API key.
