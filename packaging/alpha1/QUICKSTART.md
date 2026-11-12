# Nesticles Alpha 1 — Quick Start

Nesticles is a desktop workshop for original 2D action-platformers. A cartridge, when you have one, supplies pictures and sounds. It does not supply the game's rules. You assign those with Blueprints.

## Launch

After you run `install.sh`, open **Nesticles Alpha 1** from the application menu or from the desktop icon. You do not need Rust, Cargo, or Cursor.

From a terminal, the same program is `~/.local/share/nesticles-alpha1/nesticles`.

## 1. Create a project

The window opens on the bundled Seed Spirit sample. Choose **New** on the top bar for an empty project, or keep Seed Spirit and edit it. **Save** writes a `.json` file. The top bar says **unsaved changes** until that file matches what you see.

## 2. Import original artwork

Open **Library**. Choose a kind (character, terrain, enemy, goal, and so on) and choose **Import PNG**. Pictures must be PNG, at most 1024 pixels on a side, and at most 8 MB. Then choose **Use in Maker Engine**.

You can also drop a PNG onto the window.

## 3. Import a cartridge

Open **Cartridge** and choose **Open file…**, or drop an NES, Game Boy, Game Boy Color, Super NES, or Game Boy Advance file onto the window. Nesticles reads the header and pages uncompressed tiles. It does not read gameplay code.

On **Tiles**, set the stamp size and the tag, then **Add as part**. **Use in Maker Engine** selects that part in Courses. After the part is in the project, the cartridge file is not required to edit or play.

A file with no known header opens as a tile pager and the status line says so. A truncated NES image is refused and the project is left as it was. Files over 64 MB are refused.

## 4. Make a player

In **Courses**, select the character picture, then **Create Player Blueprint**. Name it. Choose the picture. Choose the physics preset: SMB1 - Mario, Luigi, Toad, or Peach. The picture does not choose the preset. An original sprite can use Luigi physics. A cartridge sprite can use Peach physics.

**Clear course assignment** returns that course to the older rule: the first character picture and the project physics. The blueprint itself is kept.

## 5. Paint terrain and place an enemy

Choose **New terrain brush**, assign the end, middle, and fill pictures, and drag on the grid. The tool row has single cell, freehand, horizontal line, vertical line, rectangle, erase, and select. One drag is one undo step.

Drag an enemy picture onto the grid and give it a gameplay role, such as Goomba. The picture's name is not the role.

Place a goal the same way.

## 6. Play

Choose **Play this course** or the **Play** button. Arrow keys or A and D move. Space or Z jumps. X or Shift runs. The player picture is the one on the Player Blueprint. Solid terrain holds you up. A Goomba walks, and you can stomp it or be hurt. A goal ends the run.

## 7. Save, close, and reopen

**Save** writes the project. Close the window. Launch Nesticles again and choose **Open…**. The pictures, the Player Blueprint, the enemy roles, the terrain, and the course are in that file. The original PNG and the cartridge are not required.

## Experimental Godot preview

**Export** has **Preview in Godot (Experimental)**. It needs a Godot 4.7.2 executable. If Godot is missing, Nesticles says so and the rest of the app keeps working. The preview currently steps walk, run, skid, jump, solid ground, Goombas, and goals. Pipes, items, hazards, power-ups, and other enemies stay in the Rust Play tab. The preview is not the full game.
