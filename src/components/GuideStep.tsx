export function GuideStep() {
  return (
    <section className="step guide">
      <div className="panel">
        <h1>How the workshop fits together</h1>
        <p>
          Source, extract, library, levels, overworld, play, export. A dump stays in this tab. What you keep is a
          picture, a sound, or a name you gave it. The courses and the overworld are the game. Export writes one HTML
          file that plays on Linux, Windows, and Android browsers, including touch controls.
        </p>
        <h2>Why you tag sprites</h2>
        <p>
          Classic ROMs do not label tiles as “player” or “cloud”. A tile is a few bytes of color indexes. Emberleaf can
          guess from the shape — empty, a centered blob, scenery, text-like marks, or noise — and you decide. That tag
          is what the original physics uses: terrain is solid, backgrounds scroll behind, enemies patrol, gems collect,
          thorns hurt, a gate clears the course.
        </p>
        <h2>What extraction will not do</h2>
        <p>
          Scripts in these games are custom encodings. The code tab shows ASCII strings of length 6 or more and a
          machine-code window: 6502 for NES, LR35902 for Game Boy. It is a viewer, not a script recovery. There is no
          universal physics decompiler. The jumping you play is Emberleaf’s own runtime, written for this project.
          Compressed or banked graphics need another tool. Encrypted Switch, 3DS, and Wii U images are not opened.
        </p>
        <h2>Tools to use alongside Emberleaf</h2>
        <ul>
          <li>Dazzlie — Game Boy and NES tiles to and from PNG.</li>
          <li>nesrip — NES and Super NES tilesheets. It needs its graphics database, and it understands some compression layouts.</li>
          <li>
            RetroTool / SFCRetroTools — Super NES codecs and scripts, with Godot and Tiled emitters.{' '}
            <a href="https://github.com/danielburgess/RetroTool">github.com/danielburgess/RetroTool</a>
          </li>
          <li>TileBravo — a tile editor in the YY-CHR tradition.</li>
          <li>gbgraphics — find Game Boy tiles from a screenshot.</li>
          <li>Mesen — NES, Super NES, and Game Boy tile and sprite viewers.</li>
          <li>SameBoy — a Game Boy debugger.</li>
          <li>Tiled and Godot — the path if you later want native projects, APKs, or desktop binaries.</li>
        </ul>
        <p>
          Emberleaf stays in the browser so one file plays on Linux, Windows, and Android without an emulator and
          without shipping anyone else’s game code. Pip, the tiles, and the grove tune are original.
        </p>
      </div>
    </section>
  );
}
