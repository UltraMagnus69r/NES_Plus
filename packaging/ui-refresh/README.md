# Nesticles Alpha 1 — UI Refresh

Linux x86_64. One fullscreen workshop with two sides: UnMaker Lab for pictures, Maker Engine for courses. Rust Play does not need Godot.

The download belongs in `/home/g/Downloads/`.

```sh
cd /home/g/Downloads
sha256sum -c Nesticles-Alpha1-UI-linux-x86_64.tar.gz.sha256
tar -xzf Nesticles-Alpha1-UI-linux-x86_64.tar.gz
cd Nesticles-Alpha1-UI-linux-x86_64
./nesticles
```

Or install a menu launcher, without administrator rights:

```sh
cd /home/g/Downloads/Nesticles-Alpha1-UI-linux-x86_64
./install.sh
```

Open **Nesticles Alpha 1 UI** from the KDE application launcher. `./uninstall.sh` removes that copy and leaves project files you saved elsewhere.

Flip between UnMaker Lab and Maker Engine from the top left. Play is the green button. Exit asks before discarding unsaved work.
