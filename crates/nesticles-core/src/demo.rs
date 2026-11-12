//! Original demo: a tiny iNES cartridge with drawn CHR, plus a playable workshop project.
//! No commercial sprites, music, or code.

use crate::codec::{encode_tile, Codec};
use crate::pngutil::b64_encode;
use crate::project::{Asset, AssetKind, Cell, Layer, Level, OverNode, Project, Spawn, Style};
use crate::wav::{synth_sweep, synth_tone, wav_from_samples};

pub fn demo_rom() -> Vec<u8> {
    let prg_len = 16384usize;
    let chr_len = 8192usize;
    let mut rom = vec![0u8; 16 + prg_len + chr_len];
    rom[0..4].copy_from_slice(b"NES\x1a");
    rom[4] = 1;
    rom[5] = 1;
    rom[6] = 0x01;
    rom[7] = 0x00;

    // Reset stub at CPU $8000, file offset 16. Infinite loop after init.
    let code = [
        0x78u8, // SEI
        0xD8,   // CLD
        0xA2, 0xFF, // LDX #$FF
        0x9A, // TXS
        0xA9, 0x00, // LDA #$00
        0x8D, 0x00, 0x20, // STA $2000
        0x4C, 0x0A, 0x80, // JMP $800A
    ];
    rom[16..16 + code.len()].copy_from_slice(&code);
    let message = b"SEED SPIRIT NESTICLES\0PHYSICS IS NOT STORED IN THE CARTRIDGE\0";
    rom[16 + 32..16 + 32 + message.len()].copy_from_slice(message);

    let vector = 16 + prg_len - 6;
    // NMI, RESET, IRQ all point at $8000.
    for slot in 0..3 {
        rom[vector + slot * 2] = 0x00;
        rom[vector + slot * 2 + 1] = 0x80;
    }

    let chr_at = 16 + prg_len;
    let tiles = [
        sprite_indices(),
        scenery_indices(),
        sprite_indices(),
        textish_indices(),
        noise_indices(),
    ];
    for (i, indices) in tiles.iter().enumerate() {
        let encoded = encode_tile(indices, Codec::Nes);
        let at = chr_at + (i + 1) * 16;
        rom[at..at + 16].copy_from_slice(&encoded);
    }
    rom
}

fn sprite_indices() -> Vec<u8> {
    // 8×8 sprout: clear border, filled center. Reads as sprite-like.
    let mut idx = vec![0u8; 64];
    for y in 2..6 {
        for x in 2..6 {
            idx[y * 8 + x] = if y == 2 && x == 3 { 3 } else { 1 };
        }
    }
    idx[3 * 8 + 3] = 2;
    idx[3 * 8 + 4] = 2;
    idx
}

fn scenery_indices() -> Vec<u8> {
    let mut idx = vec![1u8; 64];
    for x in 0..8 {
        idx[x] = 3;
        idx[8 + x] = 2;
    }
    idx
}

fn textish_indices() -> Vec<u8> {
    let mut idx = vec![0u8; 64];
    for y in 1..7 {
        idx[y * 8 + 2] = 1;
        idx[y * 8 + 5] = 1;
    }
    for x in 2..6 {
        idx[2 * 8 + x] = 1;
        idx[5 * 8 + x] = 1;
    }
    idx
}

fn noise_indices() -> Vec<u8> {
    let mut idx = vec![0u8; 64];
    let mut n = 0x1234u32;
    for v in &mut idx {
        n = n.wrapping_mul(1664525).wrapping_add(1013904223);
        *v = (n >> 28) as u8;
    }
    idx
}

fn color_of(ch: char) -> [u8; 4] {
    match ch {
        '#' => [42, 32, 28, 255],
        'p' => [243, 234, 215, 255],
        'e' => [224, 114, 58, 255],
        'm' => [110, 168, 74, 255],
        'd' => [36, 72, 48, 255],
        's' => [150, 96, 54, 255],
        'S' => [96, 58, 36, 255],
        'g' => [230, 186, 84, 255],
        't' => [176, 64, 78, 255],
        'b' => [126, 176, 196, 255],
        'w' => [255, 252, 246, 255],
        'k' => [32, 24, 22, 255],
        'l' => [168, 196, 112, 255],
        'n' => [22, 18, 16, 255],
        _ => [0, 0, 0, 0],
    }
}

fn blit(rows: &[&str]) -> (u32, u32, Vec<u8>) {
    let h = rows.len() as u32;
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            let c = color_of(ch);
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            if i + 4 <= rgba.len() {
                rgba[i..i + 4].copy_from_slice(&c);
            }
        }
    }
    (w, h, rgba)
}

fn asset_from(id: &str, name: &str, kind: AssetKind, rows: &[&str], cue: &str) -> Asset {
    let (w, h, rgba) = blit(rows);
    let png_base64 = {
        #[cfg(feature = "png")]
        {
            crate::pngutil::rgba_to_png(&rgba, w, h)
                .map(|png| b64_encode(&png))
                .unwrap_or_default()
        }
        #[cfg(not(feature = "png"))]
        {
            String::new()
        }
    };
    Asset {
        id: id.into(),
        name: name.into(),
        kind,
        style_id: "style-seed".into(),
        w,
        h,
        rgba_base64: b64_encode(&rgba),
        png_base64,
        wav_base64: String::new(),
        solid: kind.default_solid(),
        cue: cue.into(),
        item_tag: crate::project::ItemTag::Auto,
        block_tag: crate::project::BlockTag::Auto,
        hazard_tag: crate::project::HazardTag::Auto,
        warp: None,
    }
}

fn audio_asset(
    id: &str,
    name: &str,
    kind: AssetKind,
    samples: &[f32],
    rate: u32,
    cue: &str,
) -> Asset {
    let wav = wav_from_samples(samples, rate);
    Asset {
        id: id.into(),
        name: name.into(),
        kind,
        style_id: "style-seed".into(),
        w: 0,
        h: 0,
        rgba_base64: String::new(),
        png_base64: String::new(),
        wav_base64: b64_encode(&wav),
        solid: false,
        cue: cue.into(),
        item_tag: crate::project::ItemTag::Auto,
        block_tag: crate::project::BlockTag::Auto,
        hazard_tag: crate::project::HazardTag::Auto,
        warp: None,
    }
}

fn fill(level: &mut Level, asset: &str, layer: Layer, x0: i32, y0: i32, x1: i32, y1: i32) {
    for y in y0..y1 {
        for x in x0..x1 {
            level.cells.push(Cell {
                x,
                y,
                asset_id: asset.into(),
                layer,
                ..Cell::default()
            });
        }
    }
}

fn put(level: &mut Level, asset: &str, layer: Layer, x: i32, y: i32) {
    level.cells.push(Cell {
        x,
        y,
        asset_id: asset.into(),
        layer,
        ..Cell::default()
    });
}

pub fn demo_project() -> Project {
    let rate = 16000u32;
    let mut jump = synth_sweep(420.0, 880.0, 0.09, rate, 0.45);
    jump.extend(synth_tone(660.0, 0.04, rate, 0.15, 8.0));
    let stomp = synth_tone(140.0, 0.12, rate, 0.5, 10.0);
    let hurt = synth_sweep(520.0, 180.0, 0.16, rate, 0.4);
    let mut goal = Vec::new();
    for freq in [523.0, 659.0, 784.0] {
        goal.extend(synth_tone(freq, 0.14, rate, 0.4, 3.0));
    }
    let music = hearth_music(rate);

    let mut assets = vec![
        asset_from("pip", "Pip", AssetKind::Player, PIP, ""),
        asset_from("pebble", "Pebble", AssetKind::Enemy, PEBBLE, ""),
        asset_from("sod", "Sod", AssetKind::Terrain, SOD, ""),
        asset_from("hill", "Hill", AssetKind::Background, HILL, ""),
        asset_from("vine", "Vine", AssetKind::Foreground, VINE, ""),
        asset_from("seed", "Seed", AssetKind::Item, SEED, ""),
        asset_from("thorn", "Thorn", AssetKind::Hazard, THORN, ""),
        asset_from("nest", "Nest", AssetKind::Goal, NEST, ""),
        audio_asset("sfx-jump", "Hop", AssetKind::Sfx, &jump, rate, "jump"),
        audio_asset("sfx-stomp", "Tap", AssetKind::Sfx, &stomp, rate, "stomp"),
        audio_asset("sfx-hurt", "Stumble", AssetKind::Sfx, &hurt, rate, "hurt"),
        audio_asset("sfx-goal", "Hearth", AssetKind::Sfx, &goal, rate, "goal"),
        audio_asset(
            "music-hearth",
            "Hearth loop",
            AssetKind::Music,
            &music,
            rate,
            "music",
        ),
    ];
    let _ = &mut assets;

    let mut moss = Level {
        id: "moss".into(),
        name: "Moss Steps".into(),
        style_id: "style-seed".into(),
        w: 32,
        h: 15,
        sky: "#2a3c44".into(),
        music_id: Some("music-hearth".into()),
        player: Spawn { x: 2.0, y: 11.0 },
        cells: Vec::new(),
        enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
    };
    for x in [1, 6, 22, 28] {
        put(&mut moss, "hill", Layer::Back, x, 8);
    }
    fill(&mut moss, "sod", Layer::Main, 0, 12, 14, 15);
    fill(&mut moss, "sod", Layer::Main, 16, 12, 32, 15);
    fill(&mut moss, "sod", Layer::Main, 18, 10, 23, 11);
    put(&mut moss, "pebble", Layer::Main, 8, 11);
    put(&mut moss, "pebble", Layer::Main, 20, 9);
    put(&mut moss, "thorn", Layer::Main, 24, 11);
    put(&mut moss, "seed", Layer::Main, 21, 9);
    put(&mut moss, "nest", Layer::Main, 29, 11);
    put(&mut moss, "vine", Layer::Front, 4, 10);

    let mut lantern = Level {
        id: "lantern".into(),
        name: "Lantern Rise".into(),
        style_id: "style-seed".into(),
        w: 32,
        h: 15,
        sky: "#243038".into(),
        music_id: Some("music-hearth".into()),
        player: Spawn { x: 2.0, y: 12.0 },
        cells: Vec::new(),
        enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
    };
    for x in [2, 16, 26] {
        put(&mut lantern, "hill", Layer::Back, x, 6);
    }
    // A flat run, then a one-tile step up to the nest. Missing the step leaves you on the floor.
    fill(&mut lantern, "sod", Layer::Main, 0, 13, 32, 15);
    fill(&mut lantern, "sod", Layer::Main, 24, 12, 32, 13);
    put(&mut lantern, "pebble", Layer::Main, 8, 12);
    put(&mut lantern, "thorn", Layer::Main, 16, 12);
    put(&mut lantern, "seed", Layer::Main, 20, 12);
    put(&mut lantern, "nest", Layer::Main, 29, 11);
    put(&mut lantern, "vine", Layer::Front, 26, 10);

    Project {
        name: "Seed Spirit".into(),
        styles: vec![Style {
            id: "style-seed".into(),
            name: "Seed Spirit".into(),
            source: "original".into(),
        }],
        assets,
        levels: vec![moss, lantern],
        nodes: vec![
            OverNode {
                id: "node-moss".into(),
                name: "Moss Steps".into(),
                level_id: "moss".into(),
                x: 160.0,
                y: 240.0,
                links: vec!["node-lantern".into()],
            },
            OverNode {
                id: "node-lantern".into(),
                name: "Lantern Rise".into(),
                level_id: "lantern".into(),
                x: 460.0,
                y: 130.0,
                links: vec!["node-moss".into()],
            },
        ],
        start_node_id: "node-moss".into(),
        enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
        character_physics: crate::physics::CharacterPhysics::Smb1Mario,
        power_up_style: crate::project::PowerUpStyle::Smb1,
        format: 1,
        blueprints: Vec::new(),
    }
}

fn hearth_music(rate: u32) -> Vec<f32> {
    // Original short phrase. Not a quotation of a commercial theme.
    let notes = [
        196.0, 220.0, 262.0, 294.0, 262.0, 220.0, 247.0, 196.0, 294.0, 330.0, 294.0, 262.0, 220.0,
        196.0, 174.0, 196.0,
    ];
    let mut out = Vec::new();
    for (i, freq) in notes.iter().enumerate() {
        let dur = if i % 4 == 3 { 0.34 } else { 0.22 };
        let mut note = synth_tone(*freq, dur, rate, 0.28, 2.4);
        let bass = synth_tone(*freq * 0.5, dur, rate, 0.12, 1.6);
        for (s, b) in note.iter_mut().zip(bass.iter()) {
            *s = (*s + *b).clamp(-1.0, 1.0);
        }
        out.extend(note);
    }
    out
}

const PIP: &[&str] = &[
    "................",
    "......mm........",
    ".....mddm.......",
    "......mm........",
    "......##........",
    "....##pp##......",
    "...#pppppp#.....",
    "..#ppwppwp#.....",
    "..#ppkppkp#.....",
    "..#pppppppp#....",
    "...#ppeepp#.....",
    "....#pppp#......",
    ".....##..#......",
    "....ss..ss......",
    "...sS....Ss.....",
    "................",
];

const PEBBLE: &[&str] = &[
    "................",
    "....########....",
    "...#ssssssss#...",
    "..#sswssssws#...",
    "..#sskssssks#...",
    "..#sssssssss#...",
    "..#sssgggsss#...",
    "..#sssssssss#...",
    "...#ssssssss#...",
    "....########....",
    ".....ss..ss.....",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const SOD: &[&str] = &[
    "mmmmmmmmmmmmmmmm",
    "mlddddddddddddlm",
    "################",
    "sSsSsSsSsSsSsSsS",
    "SsSsSsSsSsSsSsSs",
    "sSsSsSsSsSsSsSsS",
    "SsSsSsSsSsSsSsSs",
    "sS#sSsSsSsS#sSsS",
    "SsSsSsSsSsSsSsSs",
    "sSsSsSsSsSsSsSsS",
    "SsSsSsSsSsSsSsSs",
    "sSsS#sSsSsSsSsSs",
    "SsSsSsSsSsSsSsSs",
    "sSsSsSsSsSsSsSsS",
    "SsSsSsSsSsSsS#sS",
    "################",
];

const HILL: &[&str] = &[
    "................",
    "................",
    "......bbbb......",
    "....bbbbbbbb....",
    "...bbddddddbb...",
    "..bbddddddddbb..",
    ".bbddmmmmmmddbb.",
    "bbddmmmmmmmmddbb",
    "ddddmmmmmmmmdddd",
    "mmmmmmmmmmmmmmmm",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const VINE: &[&str] = &[
    "......m.........",
    ".....mm.........",
    "....m.m.........",
    "...m..m.........",
    "..m...m.........",
    ".m....m.........",
    "m.....m.........",
    "......m.........",
    ".....m.m........",
    "....m...m.......",
    "...........m....",
    "..........m.....",
    ".........m......",
    "........m.......",
    ".......m........",
    "......m.........",
];

const SEED: &[&str] = &[
    "................",
    "................",
    "................",
    "......gg........",
    ".....g##g.......",
    "....g#pp#g......",
    "....#peep#......",
    "....#pppp#......",
    ".....#pp#.......",
    "......##........",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const THORN: &[&str] = &[
    "t..t....t..t....",
    ".tt..tt..tt..t..",
    "..t..tt..t..tt..",
    ".tt.tttt.tt.tt..",
    "tttttttttttttttt",
    "################",
    "sSsSsSsSsSsSsSsS",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const NEST: &[&str] = &[
    "................",
    "......gggg......",
    ".....g####g.....",
    "....g#pppp#g....",
    "...g#pggggp#g...",
    "...#ppg##gpp#...",
    "...#p######p#...",
    "...#ssssssss#...",
    "..#sSsSsSsSsS#..",
    "..#ssssssssss#..",
    "...#sSsSsSsS#...",
    "....########....",
    "......s..s......",
    ".....ss..ss.....",
    "................",
    "................",
];
