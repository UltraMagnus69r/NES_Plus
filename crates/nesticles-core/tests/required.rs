use nesticles_core::audio::{decode_brr, decode_dpcm_levels};
use nesticles_core::classify::{suggest_kind, Suggestion};
use nesticles_core::codec::{decode_tile, encode_tile, Codec};
use nesticles_core::demo::demo_rom;
use nesticles_core::detect::detect_rom;
use nesticles_core::guard::image_prompt_blocked;
use nesticles_core::physics::{Input, Sim};
use nesticles_core::project::{Asset, AssetKind, Layer, Level, Project, Spawn};
use nesticles_core::tools::execute_tool_json;

#[test]
fn nes_2bpp_roundtrip_and_ines_trainer_bit() {
    let mut indices = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            indices[y * 8 + x] = ((x + y) % 4) as u8;
        }
    }
    let nes = encode_tile(&indices, Codec::Nes);
    assert_eq!(nes.len(), 16);
    let decoded = decode_tile(&nes, 0, Codec::Nes).unwrap();
    assert_eq!(decoded, indices);
    // Separated planes: plane 1 of row 0 lives at byte 8, not byte 1.
    let gb = encode_tile(&indices, Codec::Gb);
    assert_ne!(nes, gb);

    let rom = demo_rom();
    assert_eq!(&rom[0..4], b"NES\x1a");
    let info = detect_rom(&rom, "seed.nes");
    assert!(!info.trainer);
    assert!(!info.nes2);
    assert_eq!(info.prg_start, 16);

    let mut trainer = vec![0u8; 16 + 512 + 16384];
    trainer[0..4].copy_from_slice(b"NES\x1a");
    trainer[4] = 1;
    trainer[6] = 0x04;
    let info = detect_rom(&trainer, "trainer.nes");
    assert!(info.trainer);
    assert_eq!(info.prg_start, 16 + 512);
}

#[test]
fn gb_interleaved_2bpp_roundtrip() {
    let mut indices = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            indices[y * 8 + x] = ((x * 3 + y) % 4) as u8;
        }
    }
    let gb = encode_tile(&indices, Codec::Gb);
    let decoded = decode_tile(&gb, 0, Codec::Gb).unwrap();
    assert_eq!(decoded, indices);
    // Row 0's two planes are adjacent bytes.
    assert_eq!(gb[0], {
        let mut p0 = 0u8;
        for x in 0..8 {
            if indices[x] & 1 != 0 {
                p0 |= 1 << (7 - x);
            }
        }
        p0
    });
    assert_eq!(gb[1], {
        let mut p1 = 0u8;
        for x in 0..8 {
            if indices[x] & 2 != 0 {
                p1 |= 1 << (7 - x);
            }
        }
        p1
    });
    let nes = encode_tile(&indices, Codec::Nes);
    assert_ne!(gb[1], nes[1]);
}

#[test]
fn dpcm_is_lsb_first_from_64() {
    let levels = decode_dpcm_levels(&[0x01]);
    assert_eq!(levels[0], 66);
    assert_eq!(levels[1], 64);
    let down = decode_dpcm_levels(&[0x00]);
    assert_eq!(down[0], 62);
    assert_eq!(down[7], 48);
    let clamped = decode_dpcm_levels(&[0xff, 0xff, 0xff, 0xff]);
    assert!(clamped.iter().all(|level| (0..=126).contains(level)));
    assert_eq!(*clamped.iter().max().unwrap(), 126);
    assert_eq!(clamped[0], 66);
}

#[test]
fn brr_filter_0_does_not_panic() {
    let quiet = [0x01u8, 0, 0, 0, 0, 0, 0, 0, 0];
    let samples = decode_brr(&quiet, 4);
    assert_eq!(samples.len(), 16);
    assert!(samples.iter().all(|s| *s == 0));

    // shift 1, filter 0, end bit. First nibble 1 -> ((1 << 1) >> 1) << 1 = 2.
    let known = [0x11u8, 0x10, 0, 0, 0, 0, 0, 0, 0];
    let samples = decode_brr(&known, 1);
    assert_eq!(samples[0], 2);

    // shift 15 takes the mask path and must not panic.
    let wide = [0xf1u8, 0x80, 0, 0, 0, 0, 0, 0, 0];
    let samples = decode_brr(&wide, 2);
    assert_eq!(samples.len(), 16);
}

#[test]
fn suggest_kind_empty_vs_sprite() {
    let empty = vec![0u8; 64];
    assert_eq!(suggest_kind(&empty, 8, 8), Suggestion::Empty);

    let mut sprite = vec![0u8; 64];
    for y in 2..6 {
        for x in 2..6 {
            sprite[y * 8 + x] = 1;
        }
    }
    assert_eq!(suggest_kind(&sprite, 8, 8), Suggestion::Sprite);
}

#[test]
fn physics_lands_on_a_floor_and_stomps_a_patrol_enemy() {
    let mut sim = Sim::blank();
    sim.solids.push(nesticles_core::Aabb {
        x: 0.0,
        y: 160.0,
        w: 400.0,
        h: 16.0,
    });
    sim.player.x = 30.0;
    sim.player.y = 20.0;
    sim.player.w = 12.0;
    sim.player.h = 14.0;
    sim.player.vy = 0.0;
    for _ in 0..240 {
        sim.step(&Input::default(), 1.0 / 60.0);
    }
    assert!(
        sim.player.on_ground,
        "player should land, y={}",
        sim.player.y
    );
    let feet = sim.player.y + sim.player.h;
    assert!(
        (feet - 160.0).abs() < 1.0,
        "feet {feet} should rest on the floor"
    );

    sim.enemies.push(nesticles_core::physics::Enemy {
        x: 80.0,
        y: 146.0,
        w: 14.0,
        h: 14.0,
        dir: 1,
        alive: true,
        asset_id: "pebble".into(),
    });
    // Walls keep the patrol in range while the player falls onto it.
    sim.solids.push(nesticles_core::Aabb {
        x: 64.0,
        y: 144.0,
        w: 8.0,
        h: 16.0,
    });
    sim.solids.push(nesticles_core::Aabb {
        x: 120.0,
        y: 144.0,
        w: 8.0,
        h: 16.0,
    });
    sim.player.x = 86.0;
    sim.player.y = 90.0;
    sim.player.vx = 0.0;
    sim.player.vy = 220.0;
    sim.player.on_ground = false;
    let mut stomped = false;
    for _ in 0..90 {
        sim.step(&Input::default(), 1.0 / 60.0);
        if !sim.enemies[0].alive {
            stomped = true;
            assert!(
                sim.player.vy < 0.0,
                "stomp should bounce, vy={}",
                sim.player.vy
            );
            break;
        }
    }
    assert!(stomped, "falling player should stomp the patrol enemy");
}

#[test]
fn paint_rect_changes_cells() {
    let mut project = Project::blank("test");
    project.assets.push(Asset {
        id: "ground".into(),
        name: "Ground".into(),
        kind: AssetKind::Terrain,
        style_id: String::new(),
        w: 16,
        h: 16,
        rgba_base64: String::new(),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: true,
        cue: String::new(),
    });
    project.levels.push(Level {
        id: "l1".into(),
        name: "Empty".into(),
        style_id: String::new(),
        w: 20,
        h: 15,
        sky: "#000".into(),
        music_id: None,
        player: Spawn { x: 1.0, y: 1.0 },
        cells: vec![CellStub::one()],
        enemy_ai: nesticles_core::EnemyAiStyle::Smb1Nes,
    });
    // The stub cell is a different layer so the paint is a real change either way.
    let note = execute_tool_json(
        &mut project,
        r#"{"tool":"paint_rect","level_id":"l1","asset_id":"ground","x":1,"y":2,"w":3,"h":2,"layer":"main"}"#,
    )
    .unwrap();
    assert!(note.contains("painted"));
    let cells: Vec<_> = project.levels[0]
        .cells
        .iter()
        .filter(|c| c.layer == Layer::Main && c.asset_id == "ground")
        .collect();
    assert_eq!(cells.len(), 6);
    assert!(cells.iter().any(|c| c.x == 1 && c.y == 2));
    assert!(cells.iter().any(|c| c.x == 3 && c.y == 3));
}

struct CellStub;

impl CellStub {
    fn one() -> nesticles_core::project::Cell {
        nesticles_core::project::Cell {
            x: 0,
            y: 0,
            asset_id: "ground".into(),
            layer: Layer::Back,
            ..nesticles_core::project::Cell::default()
        }
    }
}

#[test]
fn demo_courses_can_be_cleared() {
    let project = nesticles_core::demo_project();
    assert!(
        clear_course(&project, "moss"),
        "Moss Steps should be finishable"
    );
    assert!(
        clear_course(&project, "lantern"),
        "Lantern Rise should be finishable"
    );
}

fn clear_course(project: &nesticles_core::Project, level_id: &str) -> bool {
    let level = project.level(level_id).unwrap();
    let mut sim = nesticles_core::Sim::from_level(project, level);
    let mut still = 0;
    let mut last_x = sim.player.x;
    for _ in 0..60 * 45 {
        let ahead = sim.player.x + sim.player.w + 18.0;
        let feet = sim.player.y + sim.player.h + 2.0;
        let supported = sim.solids.iter().any(|s| s.contains_point(ahead, feet));
        let wall = sim
            .solids
            .iter()
            .any(|s| s.contains_point(sim.player.x + sim.player.w + 1.0, sim.player.y + 6.0));
        let hazard = sim.hazards.iter().any(|h| {
            h.x < sim.player.x + sim.player.w + 22.0
                && h.x + h.w > sim.player.x
                && (h.y - sim.player.y).abs() < 28.0
        });
        let enemy = sim.enemies.iter().any(|e| {
            e.alive
                && e.x < sim.player.x + 40.0
                && e.x + e.w > sim.player.x - 4.0
                && (e.y - sim.player.y).abs() < 24.0
        });
        let stuck = still > 12;
        let jump = sim.player.on_ground && (!supported || wall || hazard || enemy || stuck);
        let input = nesticles_core::Input {
            left: false,
            right: true,
            jump_held: jump || sim.player.vy < 0.0,
            jump_pressed: jump,
            run_held: false,
        };
        sim.step(&input, 1.0 / 60.0);
        if sim.player.x <= last_x + 0.2 {
            still += 1;
        } else {
            still = 0;
        }
        last_x = sim.player.x;
        if sim.won {
            return true;
        }
    }
    let _ = level_id;
    false
}

#[test]
fn a_second_cartridge_adds_a_style_and_parts_can_mix() {
    let mut project = nesticles_core::demo_project();
    let first = project.styles[0].id.clone();
    let second = project.adopt_style("Other Grove", "other.nes");
    assert_ne!(first, second);
    assert_eq!(project.styles.len(), 2);
    assert_eq!(
        project.adopt_style("Other Grove again", "other.nes"),
        second
    );
    project.assets.push(Asset {
        id: "crate".into(),
        name: "Crate".into(),
        kind: AssetKind::Terrain,
        style_id: second.clone(),
        w: 16,
        h: 16,
        rgba_base64: String::new(),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: true,
        cue: String::new(),
    });
    execute_tool_json(
        &mut project,
        r#"{"tool":"set_course_style","level_id":"moss","style_id":"style-seed"}"#,
    )
    .unwrap();
    execute_tool_json(
        &mut project,
        r#"{"tool":"paint_rect","level_id":"moss","asset_id":"crate","x":4,"y":11,"w":1,"h":1,"layer":"main"}"#,
    )
    .unwrap();
    let moss = project.level("moss").unwrap();
    assert_eq!(moss.style_id, "style-seed");
    assert!(moss.cells.iter().any(|cell| cell.asset_id == "sod"));
    assert!(moss.cells.iter().any(|cell| cell.asset_id == "crate"));
}

#[test]
fn image_prompt_guard_blocks_mario_and_allows_an_original_seed_spirit() {
    assert!(image_prompt_blocked("draw mario on a pipe").is_some());
    assert!(image_prompt_blocked("an original seed spirit").is_none());
}
