# Nesticles Alpha 2 — Phase 9

Linux x86_64. One fullscreen workshop with two sides. UnMaker Lab assembles sprites and named animations. Maker Engine assigns those animations to Blueprints and builds courses. Rust Play runs the game. The picture does not choose the physics.

Put the archive in your Downloads folder. `$HOME/Downloads` is that folder for the account you are using.

```sh
cd "$HOME/Downloads"
sha256sum -c Nesticles-Alpha2-Phase9-linux-x86_64.tar.gz.sha256
tar -xzf Nesticles-Alpha2-Phase9-linux-x86_64.tar.gz
cd Nesticles-Alpha2-Phase9-linux-x86_64
./nesticles
```

Or install a menu launcher, without administrator rights:

```sh
cd "$HOME/Downloads/Nesticles-Alpha2-Phase9-linux-x86_64"
./install.sh
```

Open **Nesticles Alpha 2** from the KDE application launcher. `./uninstall.sh` removes that copy and leaves project files you saved elsewhere.

This install lives beside the Alpha 1 copies. It does not replace them.

Flip between UnMaker Lab and Maker Engine from the top left. Play is the green button. Exit asks before discarding unsaved work.
