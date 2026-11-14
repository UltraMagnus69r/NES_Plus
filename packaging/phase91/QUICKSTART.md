# Quick start — Phase 9.1 test candidate

Download the archive into `$HOME/Downloads`. Do not use a system-wide install.

```sh
cd "$HOME/Downloads"
sha256sum -c Nesticles-Alpha2-Phase9.1-linux-x86_64.tar.gz.sha256
tar -xzf Nesticles-Alpha2-Phase9.1-linux-x86_64.tar.gz
cd Nesticles-Alpha2-Phase9.1-linux-x86_64
./nesticles
```

Optional user-local install, which does not replace Alpha 2 Phase 9:

```sh
./install.sh
```

Remove that install with `./uninstall.sh` from the extracted folder. The uninstaller does not delete your project files or your API key.

KDE Plasma can use the desktop file the installer writes to `~/.local/share/applications/`.

In the app:

1. Open UnMaker Lab. The home screen offers disassembly, a saved creation, or a new creation.
2. Import your own NES file. The status line shows the file digest and the payload digest. The file name is not treated as the game.
3. Review proposed objects. Approve, fail, ignore, or open the puzzle.
4. Export approved objects to Maker Engine. Ignored objects must be confirmed first. They are not deleted.
5. Press Play in Maker Engine to test the course.
