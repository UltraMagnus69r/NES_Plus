//! Write a browser build: HTML shell, wasm/js when the player was compiled, project JSON, manifest, README.

use crate::{FAILURE, JS, READY, WASM};
use nesticles_core::Project;
use std::fs;
use std::path::Path;

pub fn export_folder(dest: &Path, project: &Project) -> Result<String, String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let project_json = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
    fs::write(dest.join("project.json"), &project_json).map_err(|e| e.to_string())?;
    fs::write(dest.join("icon.png"), crate::icon::PNG).map_err(|e| e.to_string())?;
    fs::write(dest.join("manifest.webmanifest"), manifest(project)).map_err(|e| e.to_string())?;
    fs::write(dest.join("README.txt"), readme(project, READY, FAILURE))
        .map_err(|e| e.to_string())?;
    let html = if READY {
        fs::write(dest.join("nesticles_player.js"), JS).map_err(|e| e.to_string())?;
        fs::write(dest.join("nesticles_player_bg.wasm"), WASM).map_err(|e| e.to_string())?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, WASM);
        html_with_player(project, &b64)
    } else {
        html_without_player(FAILURE)
    };
    fs::write(dest.join("index.html"), html).map_err(|e| e.to_string())?;
    if READY {
        Ok(format!(
            "Exported a browser player to {}. Open index.html in Chrome, Firefox, or Edge. Adding it to an Android home screen needs https.",
            dest.display()
        ))
    } else {
        Ok(format!(
            "Wrote the project file and a note to {}. The wasm player was not embedded: {FAILURE}",
            dest.display()
        ))
    }
}

fn manifest(project: &Project) -> String {
    let (width, height) = crate::icon::png_size();
    format!(
        r##"{{
  "name": {},
  "short_name": "Nesticles",
  "start_url": "./index.html",
  "display": "standalone",
  "background_color": "#1c1614",
  "theme_color": "#e07a3d",
  "orientation": "landscape",
  "icons": [
    {{
      "src": "icon.png",
      "sizes": "{width}x{height}",
      "type": "image/png",
      "purpose": "any"
    }}
  ]
}}
"##,
        serde_json::to_string(&project.name).unwrap_or_else(|_| "\"Nesticles\"".into())
    )
}

fn readme(project: &Project, ready: bool, failure: &str) -> String {
    let player = if ready {
        "This folder includes the wasm player. Open index.html directly in Chrome, Firefox, or Edge on Linux, Windows, or Android. The wasm and the project are inlined in the page so a file open does not need a server."
    } else {
        "The wasm player was not embedded in this export."
    };
    format!(
        "\
{name}
{player}
{failure}

Controls: Arrow keys or A/D to move, Z, W, Up, or Space to jump (A). Run is Left Shift, X, or the Run button. The on-screen buttons work with a finger.

Add to Home Screen on Android needs a site served over https. A file open is enough to play; the home-screen install is the extra step that needs https. icon.png in this folder is the tab icon and the home-screen icon.

This build is a browser player, not an Android package.
",
        name = project.name,
        player = player,
        failure = if ready { String::new() } else { format!("Build note: {failure}\n") }
    )
}

fn icon_links() -> String {
    let (width, height) = crate::icon::png_size();
    format!(
        r#"<link rel="icon" type="image/png" sizes="{width}x{height}" href="icon.png">
  <link rel="apple-touch-icon" sizes="{width}x{height}" href="icon.png">"#
    )
}

fn html_with_player(project: &Project, wasm_b64: &str) -> String {
    let project_js =
        serde_json::to_string(&serde_json::to_string(project).unwrap_or_else(|_| "{}".into()))
            .unwrap_or_else(|_| "\"{}\"".into())
            .replace('<', "\\u003c");
    let wasm_js = serde_json::to_string(wasm_b64)
        .unwrap_or_else(|_| "\"\"".into())
        .replace('<', "\\u003c");
    let icons = icon_links();
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
  <meta name="theme-color" content="#e07a3d">
  <title>{title}</title>
  {icons}
  <link rel="manifest" href="manifest.webmanifest">
  <style>
    html, body {{ margin: 0; height: 100%; background: #1c1614; color: #f4ece0; font-family: Georgia, "Liberation Serif", serif; }}
    body {{ display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 12px; box-sizing: border-box; }}
    h1 {{ font-weight: 600; font-size: 28px; margin: 0; letter-spacing: 0.02em; }}
    p {{ margin: 0; max-width: 640px; text-align: center; line-height: 1.4; }}
    canvas {{ image-rendering: pixelated; image-rendering: crisp-edges; background: #24343c; touch-action: none; width: min(100%, 960px); height: auto; border: 2px solid #5a3a2a; }}
    #touch {{ display: flex; gap: 10px; width: min(100%, 960px); }}
    button {{ flex: 1; font: 600 18px Georgia, serif; background: #3a2c26; color: #f4ece0; border: 1px solid #e07a3d; border-radius: 10px; padding: 16px 8px; touch-action: none; }}
    button:active {{ background: #e07a3d; color: #1c1614; }}
    #note {{ opacity: 0.8; font-size: 14px; }}
  </style>
</head>
<body>
  <h1>{title}</h1>
  <canvas id="screen" width="480" height="270"></canvas>
  <div id="touch">
    <button id="left" type="button">Left</button>
    <button id="right" type="button">Right</button>
    <button id="jump" type="button">Jump</button>
    <button id="run" type="button">Run</button>
  </div>
  <p id="note">Arrow keys or A/D. Jump (A) is Z, W, Up, or Space. Run is Left Shift, X, or the Run button. File open is enough to play. Add to Home Screen needs https.</p>
  <script src="nesticles_player.js"></script>
  <script>
    const projectText = {project_js};
    const wasmB64 = {wasm_js};
    const keys = {{ left: false, right: false, up: false, down: false, jump: false, jumpPressed: false, run: false }};
    function setKey(name, down) {{
      if (name === "jump" && down && !keys.jump) keys.jumpPressed = true;
      keys[name] = down;
    }}
    window.addEventListener("keydown", (e) => {{
      const k = e.key;
      if (k === "ArrowLeft" || k === "a" || k === "A") setKey("left", true);
      if (k === "ArrowRight" || k === "d" || k === "D") setKey("right", true);
      if (k === "ArrowDown" || k === "s" || k === "S") setKey("down", true);
      if (k === "ArrowUp" || k === "w" || k === "W") setKey("up", true);
      if (k === "Shift" || k === "x" || k === "X") setKey("run", true);
      if (k === "ArrowUp" || k === "w" || k === "W" || k === "z" || k === "Z" || k === " ") {{
        setKey("jump", true);
        e.preventDefault();
      }}
    }});
    window.addEventListener("keyup", (e) => {{
      const k = e.key;
      if (k === "ArrowLeft" || k === "a" || k === "A") setKey("left", false);
      if (k === "ArrowRight" || k === "d" || k === "D") setKey("right", false);
      if (k === "ArrowDown" || k === "s" || k === "S") setKey("down", false);
      if (k === "ArrowUp" || k === "w" || k === "W") setKey("up", false);
      if (k === "Shift" || k === "x" || k === "X") setKey("run", false);
      if (k === "ArrowUp" || k === "w" || k === "W" || k === "z" || k === "Z" || k === " ") setKey("jump", false);
    }});
    function hold(id, name) {{
      const el = document.getElementById(id);
      const down = (e) => {{ e.preventDefault(); setKey(name, true); }};
      const up = (e) => {{ e.preventDefault(); setKey(name, false); }};
      el.addEventListener("pointerdown", down);
      el.addEventListener("pointerup", up);
      el.addEventListener("pointercancel", up);
      el.addEventListener("pointerleave", up);
    }}
    hold("left", "left");
    hold("right", "right");
    hold("jump", "jump");
    hold("run", "run");
    const audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    const loops = {{}};
    window.nesticlesPlayWav = function (b64, looped) {{
      try {{
        if (audioCtx.state === "suspended") audioCtx.resume();
        const bin = atob(b64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        audioCtx.decodeAudioData(bytes.buffer.slice(0)).then((buf) => {{
          const src = audioCtx.createBufferSource();
          src.buffer = buf;
          src.loop = !!looped;
          src.connect(audioCtx.destination);
          if (looped) {{
            if (loops.music) {{ try {{ loops.music.stop(); }} catch (e) {{}} }}
            loops.music = src;
          }}
          src.start();
        }}).catch(() => {{}});
      }} catch (e) {{}}
    }};
    function b64ToBytes(b64) {{
      const bin = atob(b64);
      const out = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
      return out;
    }}
    const canvas = document.getElementById("screen");
    canvas.addEventListener("pointerdown", (e) => {{
      const r = canvas.getBoundingClientRect();
      const x = (e.clientX - r.left) * (canvas.width / r.width);
      const y = (e.clientY - r.top) * (canvas.height / r.height);
      if (window.wasm_bindgen && wasm_bindgen.pointer) wasm_bindgen.pointer(x, y, true);
    }});
    let last = performance.now();
    function loop(now) {{
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const pressed = keys.jumpPressed;
      keys.jumpPressed = false;
      if (window.wasm_bindgen && wasm_bindgen.set_run) wasm_bindgen.set_run(!!keys.run);
      if (window.wasm_bindgen && wasm_bindgen.set_vertical) wasm_bindgen.set_vertical(!!keys.up, !!keys.down);
      if (window.wasm_bindgen && wasm_bindgen.frame) {{
        wasm_bindgen.frame(keys.left, keys.right, keys.jump, pressed, dt);
      }}
      requestAnimationFrame(loop);
    }}
    function fail(err) {{
      document.getElementById("note").textContent = "Player failed to start: " + err;
    }}
    const boot = wasm_bindgen(b64ToBytes(wasmB64));
    Promise.resolve(boot).then(() => {{
      wasm_bindgen.start_game(projectText);
      requestAnimationFrame(loop);
    }}).catch(fail);
  </script>
</body>
</html>
"##,
        title = project.name,
        icons = icons,
        project_js = project_js,
        wasm_js = wasm_js
    )
}

fn html_without_player(failure: &str) -> String {
    let safe = failure.replace('<', "");
    let icons = icon_links();
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Nesticles</title>
  {icons}
  <link rel="manifest" href="manifest.webmanifest">
</head>
<body style="background:#1c1614;color:#f4ece0;font-family:Georgia,serif;padding:24px">
  <h1>Nesticles</h1>
  <p>The browser player was not embedded in this export.</p>
  <p>{safe}</p>
  <p>project.json is the game. Open it in the Nesticles desktop app to play there.</p>
</body>
</html>
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nesticles_core::demo_project;

    #[test]
    fn export_embeds_the_program_icon() {
        let dest =
            std::env::temp_dir().join(format!("nesticles-icon-export-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);
        export_folder(&dest, &demo_project()).expect("export");
        let png = fs::read(dest.join("icon.png")).expect("icon.png");
        assert_eq!(png, crate::icon::PNG);
        let html = fs::read_to_string(dest.join("index.html")).expect("index.html");
        assert!(html.contains("rel=\"icon\""));
        assert!(html.contains("rel=\"apple-touch-icon\""));
        assert!(html.contains("href=\"icon.png\""));
        assert!(!html.contains("href=\"http"));
        let manifest_text =
            fs::read_to_string(dest.join("manifest.webmanifest")).expect("manifest");
        let manifest: serde_json::Value =
            serde_json::from_str(&manifest_text).expect("manifest json");
        assert_eq!(manifest["icons"][0]["src"], "icon.png");
        assert_eq!(manifest["icons"][0]["type"], "image/png");
        let (width, height) = crate::icon::png_size();
        assert_eq!(manifest["icons"][0]["sizes"], format!("{width}x{height}"));
        let _ = fs::remove_dir_all(&dest);
    }
}
