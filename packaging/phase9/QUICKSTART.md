# Nesticles Alpha 2 — Quick Start

Nesticles is one local workshop. UnMaker Lab makes pictures and animations. Maker Engine gives those pictures gameplay. A picture of an enemy does not become an enemy until you assign a Blueprint.

## Launch

The window opens fullscreen. From a terminal, after `install.sh`, the program is:

`~/.local/share/nesticles-alpha2-phase9/nesticles`

You can also run `./nesticles` from the extracted folder.

## 1. Open or save a project

The window opens on the bundled Seed Spirit sample. **Save** writes a `.json` file. **Menu** has New, Open, and Exit to Desktop. Exit asks before discarding unsaved sprite or course edits.

## 2. Assemble a sprite from cartridge tiles

Open **UnMaker Lab**. Choose **Import ROM**, or type a path and choose **Load path**, or choose **Load demo cartridge**. On **Tiles**, choose **2×2**, click a tile, then **Assemble selected tiles**. The cartridge file is not copied into the project.

On **Sprite Assembly**, drag a piece, nudge it, mirror it, or change its layer. **Undo** and **Redo** restore the sprite.

## 3. Animate

Choose **New animation**, type a state name such as `idle`, `walk`, or `jump`, and choose **New state**. The timeline can add, duplicate, delete, and reorder frames. The tick slider stores duration at 60 Hz. Play, pause, restart, and the step buttons preview that timing. Half speed and double speed do not rewrite the stored ticks.

**Use standard fallbacks** only when you want Run to use Walk, and Walk to use Idle. A missing state keeps the still picture and is listed in the inspector.

**Slice selected sheet** cuts the selected PNG using the frame width, height, margin, and gap in the inspector. A picture with one frame can stay a still.

## 4. Use it in Maker Engine

Choose **Open in Maker Engine**, or **Maker Engine** on the top left. The animation is already in the same project.

In Courses, create a Player Blueprint if the course does not have one, then choose the animation. The physics preset stays put.

Select a placed enemy. The inspector can assign an animation to that Blueprint. The enemy's brain and collision stay put.

## 5. Play

Press **Play**. The player uses idle, walk, jump, fall, and hurt when the simulation is in those states. An enemy uses walk while it is alive and defeat for a short moment after a stomp. Other state names are saved for later and are not claimed as runtime behavior.

Save, quit, and open the same file. The sprite, the frames, and the Blueprint assignment come back without the original cartridge or PNG.
