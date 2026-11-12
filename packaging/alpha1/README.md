# Nesticles Alpha 1 — Linux x86_64

This folder is the portable Alpha 1 editor. It is not a ROM player and it does not compile cartridges. UnMaker Lab keeps pictures and sounds. Maker Engine keeps gameplay.

## Install (no administrator)

```sh
tar -xzf Nesticles-Alpha1-linux-x86_64.tar.gz
cd Nesticles-Alpha1-linux-x86_64
./install.sh
```

`install.sh` copies the program to `~/.local/share/nesticles-alpha1`, adds an application menu entry, and copies a launcher onto the XDG desktop when that folder exists. It does not need sudo, a Rust toolchain, or Cursor.

Open **Nesticles Alpha 1** from the application menu or the desktop icon.

## Run without installing

```sh
cd Nesticles-Alpha1-linux-x86_64
./nesticles
```

The executable does not need the source repository and does not need to be started from a Cargo directory.

## What the host must already have

- glibc 2.39 or newer (Fedora 40 and current Bazzite meet this)
- GTK 3 (`libgtk-3.so.0`) and GLib (`libglib-2.0.so.0`), used for the window and the file chooser
- A desktop session with OpenGL (the system's Mesa driver)

If the program says `libgtk-3.so.0: cannot open shared object file`, GTK 3 is missing. Do not rebase the OS to fix that.

Rust Play works with no Godot install and no network. **Preview in Godot (Experimental)** looks for a Godot 4.7.2 executable and explains the gap if it is missing. This package does not include the Godot editor.

## Uninstall

```sh
./uninstall.sh
```

That removes the user-local copy and the launchers. Project `.json` files you saved stay where you put them.

## Read next

- `QUICKSTART.md` — create a project, import art, assign a player, paint, play, save
- `LIMITATIONS.md` — cartridge formats, Godot subset, what is not in this alpha
