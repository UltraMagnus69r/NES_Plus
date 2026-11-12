//! Play-tab baselines. These call [`crate::physics::Sim::tick_play`], the path the
//! Play tab uses, on small courses with fixed inputs and tick counts.
//!
//! `Sim::from_level` still publishes the float body until the first `tick_play`.
//! Assertions about the 1080p body are taken after that tick. Gameplay constants
//! are not adjusted for these tests.

use crate::demo::demo_project;
use crate::enemy_ai::EnemyAiStyle;
use crate::physics::{CharacterPhysics, Input, Sim, TILE};
use crate::project::{
    Asset, AssetKind, BlockTag, Cell, HazardTag, ItemTag, Layer, Level, PowerUpStyle, Project,
    Spawn,
};

fn asset(id: &str, name: &str, kind: AssetKind) -> Asset {
    Asset {
        id: id.into(),
        name: name.into(),
        kind,
        style_id: String::new(),
        w: 16,
        h: 16,
        rgba_base64: "AQIDBA==".into(),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: kind == AssetKind::Terrain,
        cue: String::new(),
        item_tag: ItemTag::Auto,
        block_tag: BlockTag::Auto,
        hazard_tag: HazardTag::Auto,
        warp: None,
    }
}

fn cell(id: &str, x: i32, y: i32) -> Cell {
    Cell {
        x,
        y,
        asset_id: id.into(),
        layer: Layer::Main,
        ..Cell::default()
    }
}

fn ground_row() -> Vec<Cell> {
    (0..24).map(|x| cell("ground", x, 13)).collect()
}

fn course(cells: Vec<Cell>, px: f32, py: f32) -> Level {
    Level {
        id: "course".into(),
        name: "Play path".into(),
        style_id: String::new(),
        w: 24,
        h: 15,
        sky: "#24343c".into(),
        music_id: None,
        player: Spawn { x: px, y: py },
        cells,
        enemy_ai: EnemyAiStyle::Smb1Nes,
        entrances: Vec::new(),
        player_blueprint_id: None,
    }
}

fn stage(character: CharacterPhysics, assets: Vec<Asset>, level: Level) -> (Project, Sim) {
    let mut project = Project::blank("play-path");
    project.character_physics = character;
    project.power_up_style = PowerUpStyle::Smb1;
    project.enemy_ai = EnemyAiStyle::Smb1Nes;
    project.assets = assets;
    project.levels = vec![level];
    let sim = Sim::from_level(&project, &project.levels[0]);
    (project, sim)
}

fn parts() -> Vec<Asset> {
    vec![
        asset("ground", "Ground", AssetKind::Terrain),
        asset("goomba", "Goomba", AssetKind::Enemy),
        asset("goal", "Flag", AssetKind::Goal),
        asset("thorn", "Thorn", AssetKind::Hazard),
        asset("shroom", "Mushroom", AssetKind::Item),
    ]
}

fn play(sim: &mut Sim, preset: CharacterPhysics, input: &Input) {
    sim.tick_play(preset, PowerUpStyle::Smb1, input);
}

fn near(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 0.001
}

#[test]
fn mario_walk_run_jump_gravity_and_ground_contact() {
    let (project, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(ground_row(), 2.0, 12.0),
    );
    let preset = project.character_physics;
    // Float snapshot until the play tick. The 1080p body is 16px and cell-aligned.
    assert!(sim.player.on_ground);
    assert_eq!(sim.player.h, 14.0);
    assert!(near(sim.player.y, 12.0 * TILE + 2.0));

    let walk = Input {
        right: true,
        ..Input::default()
    };
    let mut max_walk = 0.0f32;
    for _ in 0..80 {
        play(&mut sim, preset, &walk);
        max_walk = max_walk.max(sim.player.vx);
        assert!(
            sim.player.vx <= 90.0 + 0.001,
            "walk speed {} exceeded the cap",
            sim.player.vx
        );
    }
    assert!(near(sim.player.vx, 90.0), "vx {}", sim.player.vx);
    assert!(near(max_walk, 90.0));
    assert!(sim.player.on_ground);
    assert!(near(sim.player.y, 192.0), "y {}", sim.player.y);
    assert!(near(sim.player.h, 16.0));
    assert!(near(sim.player.y + sim.player.h, 208.0));
    assert!(near(sim.player.x, 124.4296875), "x {}", sim.player.x);
    assert_eq!(sim.player.facing, 1);

    let (project, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(ground_row(), 2.0, 12.0),
    );
    let run = Input {
        right: true,
        run_held: true,
        ..Input::default()
    };
    for i in 0..45 {
        play(&mut sim, project.character_physics, &run);
        assert!(
            sim.player.vx <= 150.0 + 0.001,
            "run speed {} exceeded the cap at tick {}",
            sim.player.vx,
            i + 1
        );
    }
    assert!(near(sim.player.vx, 150.0), "vx {}", sim.player.vx);
    assert!(sim.player.vx > 90.0);
    assert!(near(sim.player.x, 91.607421875), "x {}", sim.player.x);
    assert!(sim.player.on_ground);
    assert_eq!(sim.player.facing, 1);

    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(ground_row(), 2.0, 12.0),
    );
    play(
        &mut sim,
        preset,
        &Input {
            left: true,
            ..Input::default()
        },
    );
    assert!(sim.player.vx < 0.0, "vx {}", sim.player.vx);
    assert_eq!(sim.player.facing, -1);

    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(ground_row(), 2.0, 12.0),
    );
    play(
        &mut sim,
        preset,
        &Input {
            jump_pressed: true,
            jump_held: true,
            ..Input::default()
        },
    );
    assert!(sim.events.jump);
    assert!(!sim.player.on_ground);
    assert!(near(sim.player.vy, -232.5), "jump vy {}", sim.player.vy);
    assert!(near(sim.player.y, 188.125), "y {}", sim.player.y);
    play(
        &mut sim,
        preset,
        &Input {
            jump_held: true,
            ..Input::default()
        },
    );
    // Hold gravity is 7.5 course-px/s per tick, so -232.5 becomes -225.
    assert!(near(sim.player.vy, -225.0), "held vy {}", sim.player.vy);
    play(&mut sim, preset, &Input::default());
    // Releasing jump switches to normal gravity, +26.25 course-px/s.
    assert!(
        near(sim.player.vy, -198.75),
        "released vy {}",
        sim.player.vy
    );
    assert!(!sim.player.on_ground);

    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(ground_row(), 3.0, 4.0),
    );
    assert!(!sim.player.on_ground);
    let start_y = sim.player.y;
    let mut landed = None;
    for i in 0..180 {
        play(&mut sim, preset, &Input::default());
        if sim.player.on_ground {
            landed = Some(i + 1);
            break;
        }
    }
    assert_eq!(landed, Some(34));
    assert!(sim.player.y > start_y);
    assert!(near(sim.player.y, 192.0), "y {}", sim.player.y);
    assert!(near(sim.player.h, 16.0));
    assert!(near(sim.player.y + sim.player.h, 13.0 * TILE));
    assert!(near(sim.player.vy, 0.0));
    play(&mut sim, preset, &Input::default());
    assert!(sim.player.on_ground);
    assert!(near(sim.player.y, 192.0));
}

#[test]
fn luigi_jumps_higher_and_accelerates_slower_than_mario() {
    let mario = sample_jump_and_walk(CharacterPhysics::Smb1Mario);
    let luigi = sample_jump_and_walk(CharacterPhysics::Smb1Luigi);
    assert!(
        luigi.jump_vy < mario.jump_vy,
        "luigi {} mario {}",
        luigi.jump_vy,
        mario.jump_vy
    );
    assert!(near(luigi.jump_vy, -260.6673177), "vy {}", luigi.jump_vy);
    assert!(near(mario.jump_vy, -232.5));
    assert!(
        luigi.walk_10 < mario.walk_10,
        "luigi {} mario {}",
        luigi.walk_10,
        mario.walk_10
    );
    assert!(near(luigi.walk_10, 16.015625));
    assert!(near(mario.walk_10, 22.265625));
    // Same walk cap; Luigi's difference here is the slower approach and the jump.
    assert!(near(luigi.walk_cap, 90.0));
    assert!(near(mario.walk_cap, 90.0));
}

#[test]
fn toad_reaches_a_higher_walk_speed_and_a_lower_jump() {
    let mario = sample_jump_and_walk(CharacterPhysics::Smb1Mario);
    let toad = sample_jump_and_walk(CharacterPhysics::Smb1Toad);
    assert!(toad.walk_10 > mario.walk_10);
    assert!(toad.walk_cap > mario.walk_cap);
    assert!(near(toad.walk_10, 31.1848958), "vx {}", toad.walk_10);
    assert!(near(toad.walk_cap, 105.0));
    assert!(
        toad.jump_vy > mario.jump_vy,
        "toad {} mario {}",
        toad.jump_vy,
        mario.jump_vy
    );
    assert!(near(toad.jump_vy, -217.6660156));
}

#[test]
fn peach_hover_holds_near_zero_while_mario_is_falling() {
    let tick = 46;
    let mario = hover_sample(CharacterPhysics::Smb1Mario, tick);
    let peach = hover_sample(CharacterPhysics::Smb1Peach, tick);
    assert!(!peach.on_ground);
    assert!(
        peach.vy.abs() < 4.0,
        "peach hover vy {} at tick {tick}",
        peach.vy
    );
    assert!(near(peach.vy.abs(), 10.0 / 3.0), "vy {}", peach.vy);
    assert!(!mario.on_ground);
    assert!(mario.vy > 200.0, "mario should be falling, vy {}", mario.vy);
    assert!(near(
        sample_jump_and_walk(CharacterPhysics::Smb1Peach).walk_cap,
        80.0
    ));
}

struct MotionSample {
    walk_10: f32,
    walk_cap: f32,
    jump_vy: f32,
}

fn sample_jump_and_walk(preset: CharacterPhysics) -> MotionSample {
    let (_, mut sim) = stage(preset, parts(), course(ground_row(), 2.0, 12.0));
    let walk = Input {
        right: true,
        ..Input::default()
    };
    for _ in 0..10 {
        play(&mut sim, preset, &walk);
    }
    let walk_10 = sim.player.vx;
    let (_, mut sim) = stage(preset, parts(), course(ground_row(), 2.0, 12.0));
    for _ in 0..120 {
        play(&mut sim, preset, &walk);
    }
    let walk_cap = sim.player.vx;
    let (_, mut sim) = stage(preset, parts(), course(ground_row(), 2.0, 12.0));
    play(
        &mut sim,
        preset,
        &Input {
            jump_pressed: true,
            jump_held: true,
            ..Input::default()
        },
    );
    MotionSample {
        walk_10,
        walk_cap,
        jump_vy: sim.player.vy,
    }
}

struct HoverSample {
    vy: f32,
    on_ground: bool,
}

fn hover_sample(preset: CharacterPhysics, ticks: i32) -> HoverSample {
    let (_, mut sim) = stage(preset, parts(), course(ground_row(), 2.0, 12.0));
    for i in 0..ticks {
        play(
            &mut sim,
            preset,
            &Input {
                jump_pressed: i == 0,
                jump_held: true,
                ..Input::default()
            },
        );
    }
    HoverSample {
        vy: sim.player.vy,
        on_ground: sim.player.on_ground,
    }
}

#[test]
fn goomba_walks_and_reverses_through_tick_play() {
    let mut cells = ground_row();
    cells.push(cell("goomba", 8, 12));
    cells.push(cell("ground", 6, 12));
    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(cells, 2.0, 12.0),
    );
    assert_eq!(sim.enemies.len(), 1);
    // Float patrol parity before the play tick. The 1080p goomba then faces the player.
    assert_eq!(sim.enemies[0].dir, 1);
    play(&mut sim, CharacterPhysics::Smb1Mario, &Input::default());
    assert_eq!(sim.enemies[0].dir, -1);
    assert!(near(sim.enemies[0].x, 129.5), "x {}", sim.enemies[0].x);
    assert!(near(sim.enemies[0].y, 192.0));
    assert!(sim.enemies[0].alive);
    for _ in 1..36 {
        play(&mut sim, CharacterPhysics::Smb1Mario, &Input::default());
    }
    assert_eq!(sim.enemies[0].dir, -1);
    assert!(near(sim.enemies[0].x, 112.0), "x {}", sim.enemies[0].x);
    let against_wall = sim.enemies[0].x;
    for _ in 36..41 {
        play(&mut sim, CharacterPhysics::Smb1Mario, &Input::default());
    }
    assert_eq!(sim.enemies[0].dir, 1, "goomba should reverse off the wall");
    assert!(sim.enemies[0].x > against_wall);
    assert!(near(sim.enemies[0].x, 114.0), "x {}", sim.enemies[0].x);
    assert!(sim.enemies[0].alive);
}

#[test]
fn falling_onto_a_goomba_stomps_it() {
    let mut cells = ground_row();
    cells.push(cell("goomba", 5, 12));
    cells.push(cell("ground", 4, 12));
    cells.push(cell("ground", 6, 12));
    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(cells, 5.0, 10.0),
    );
    let mut stomped_at = None;
    for i in 0..40 {
        play(&mut sim, CharacterPhysics::Smb1Mario, &Input::default());
        if sim.events.stomp {
            stomped_at = Some(i + 1);
            break;
        }
        assert!(
            !sim.events.hurt,
            "tick {} treated the fall as damage",
            i + 1
        );
    }
    assert_eq!(stomped_at, Some(9));
    assert!(!sim.enemies[0].alive);
    assert!(near(sim.player.vy, -240.0), "stomp vy {}", sim.player.vy);
    assert!(!sim.player.on_ground);
}

#[test]
fn walking_into_a_goomba_damages_small_mario() {
    let mut cells = ground_row();
    cells.push(cell("goomba", 6, 12));
    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(cells, 2.0, 12.0),
    );
    let spawn_x = sim.player.x;
    let step = Input {
        right: true,
        ..Input::default()
    };
    let mut hurt_at = None;
    for i in 0..80 {
        play(&mut sim, CharacterPhysics::Smb1Mario, &step);
        if sim.events.hurt {
            hurt_at = Some(i + 1);
            break;
        }
        assert!(!sim.events.stomp);
    }
    assert_eq!(hurt_at, Some(41));
    assert!(sim.enemies[0].alive);
    assert!(near(sim.player.x, spawn_x), "respawn x {}", sim.player.x);
    assert!(near(sim.player.vy, 0.0));
}

#[test]
fn goal_is_reached_on_a_fixed_tick() {
    let mut cells = ground_row();
    cells.push(cell("goal", 6, 12));
    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(cells, 2.0, 12.0),
    );
    let step = Input {
        right: true,
        ..Input::default()
    };
    let mut won_at = None;
    for i in 0..80 {
        play(&mut sim, CharacterPhysics::Smb1Mario, &step);
        if sim.won {
            won_at = Some(i + 1);
            assert!(sim.events.goal);
            assert!(near(sim.player.vx, 0.0));
            assert!(near(sim.player.x, 85.4296875), "x {}", sim.player.x);
            break;
        }
        assert!(!sim.events.goal);
    }
    assert_eq!(won_at, Some(54));
    play(&mut sim, CharacterPhysics::Smb1Mario, &step);
    assert!(sim.won);
}

#[test]
fn hazard_defeats_small_mario_and_a_mushroom_changes_form_first() {
    let mut cells = ground_row();
    cells.push(cell("thorn", 6, 12));
    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(cells, 2.0, 12.0),
    );
    let spawn_x = sim.player.x;
    let step = Input {
        right: true,
        ..Input::default()
    };
    let mut hurt_at = None;
    for i in 0..80 {
        play(&mut sim, CharacterPhysics::Smb1Mario, &step);
        if sim.events.hurt {
            hurt_at = Some(i + 1);
            break;
        }
    }
    assert_eq!(hurt_at, Some(54));
    assert!(near(sim.player.x, spawn_x), "x {}", sim.player.x);
    assert!(near(sim.player.h, 16.0));
    assert!(near(sim.player.y, 192.0));

    let mut cells = ground_row();
    cells.push(cell("shroom", 2, 12));
    cells.push(cell("thorn", 8, 12));
    let (_, mut sim) = stage(
        CharacterPhysics::Smb1Mario,
        parts(),
        course(cells, 2.0, 12.0),
    );
    assert_eq!(sim.player.h, 14.0);
    play(&mut sim, CharacterPhysics::Smb1Mario, &Input::default());
    assert!(sim.events.collect);
    assert!(near(sim.player.h, 32.0), "super h {}", sim.player.h);
    assert!(near(sim.player.y, 176.0), "y {}", sim.player.y);
    assert!(sim.player.on_ground);
    assert!(near(sim.player.vy, 0.0));
    let mut hurt_at = None;
    for i in 0..160 {
        play(
            &mut sim,
            CharacterPhysics::Smb1Mario,
            &Input {
                right: true,
                ..Input::default()
            },
        );
        if i == 20 {
            assert!(near(sim.player.h, 32.0));
            assert!(near(sim.player.x, 42.572266), "x {}", sim.player.x);
        }
        if sim.events.hurt {
            hurt_at = Some(i + 2);
            break;
        }
    }
    assert_eq!(hurt_at, Some(76));
    // Super damage shrinks in place. It does not respawn at the course start.
    assert!(near(sim.player.h, 16.0), "h {}", sim.player.h);
    assert!(sim.player.x > 80.0, "x {}", sim.player.x);
    assert!(near(sim.player.x, 116.92969), "x {}", sim.player.x);
    assert!(near(sim.player.y, 192.0));
}

#[test]
fn one_frame_asset_and_existing_project_round_trip() {
    let frame = asset("sprout", "Sprout", AssetKind::Player);
    assert_eq!(frame.w, 16);
    assert_eq!(frame.h, 16);
    assert!(!frame.rgba_base64.is_empty());
    let json = serde_json::to_string_pretty(&frame).unwrap();
    let loaded: Asset = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.id, "sprout");
    assert_eq!(loaded.name, "Sprout");
    assert_eq!(loaded.kind, AssetKind::Player);
    assert_eq!(loaded.rgba_base64, frame.rgba_base64);
    assert_eq!(loaded.png_base64, "");
    assert_eq!(loaded.w, 16);
    assert_eq!(loaded.h, 16);

    let mut cells = ground_row();
    cells.push(cell("goomba", 4, 12));
    let (project, _) = stage(
        CharacterPhysics::Smb1Luigi,
        vec![
            frame,
            asset("ground", "Ground", AssetKind::Terrain),
            asset("goomba", "Goomba", AssetKind::Enemy),
        ],
        course(cells, 2.0, 12.0),
    );
    let saved = serde_json::to_string_pretty(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.name, project.name);
    assert_eq!(loaded.assets.len(), 3);
    assert_eq!(loaded.assets[0].rgba_base64, "AQIDBA==");
    assert_eq!(loaded.levels.len(), 1);
    assert_eq!(loaded.levels[0].cells.len(), project.levels[0].cells.len());
    assert_eq!(loaded.levels[0].player.x, 2.0);
    assert_eq!(loaded.character_physics, CharacterPhysics::Smb1Luigi);
    assert_eq!(loaded.power_up_style, PowerUpStyle::Smb1);
    assert_eq!(loaded.enemy_ai, EnemyAiStyle::Smb1Nes);
    assert_eq!(serde_json::to_string_pretty(&loaded).unwrap(), saved);
}

#[test]
fn seed_spirit_demo_project_loads() {
    let project = demo_project();
    let saved = serde_json::to_string_pretty(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.name, "Seed Spirit");
    assert_eq!(loaded.styles.len(), 1);
    assert_eq!(loaded.styles[0].id, "style-seed");
    assert_eq!(loaded.assets.len(), project.assets.len());
    assert!(loaded.assets.iter().any(|asset| {
        asset.id == "pebble" && asset.kind == AssetKind::Enemy && asset.name == "Pebble"
    }));
    assert_eq!(loaded.levels.len(), 2);
    assert_eq!(loaded.levels[0].id, "moss");
    assert_eq!(loaded.levels[1].id, "lantern");
    assert!(loaded.levels[0]
        .cells
        .iter()
        .any(|cell| cell.asset_id == "pebble"));
    assert_eq!(loaded.start_node_id, "node-moss");
    assert_eq!(loaded.character_physics, CharacterPhysics::Smb1Mario);
    assert_eq!(loaded.power_up_style, PowerUpStyle::Smb1);
    assert_eq!(loaded.enemy_ai, EnemyAiStyle::Smb1Nes);
    assert_eq!(loaded.levels[0].enemy_ai, EnemyAiStyle::Smb1Nes);
    let pebble = loaded
        .assets
        .iter()
        .find(|asset| asset.id == "pebble")
        .unwrap();
    assert!(!pebble.rgba_base64.is_empty());
    assert_eq!(serde_json::to_string_pretty(&loaded).unwrap(), saved);
}

#[test]
fn gameplay_presets_deserialize_and_round_trip() {
    let cases = [
        ("smb1_mario", CharacterPhysics::Smb1Mario),
        ("smb1_luigi", CharacterPhysics::Smb1Luigi),
        ("smb1_toad", CharacterPhysics::Smb1Toad),
        ("smb1_peach", CharacterPhysics::Smb1Peach),
        ("smb1_nes", CharacterPhysics::Smb1Mario),
        ("nesticles", CharacterPhysics::Smb1Mario),
    ];
    for (text, expect) in cases {
        let parsed: CharacterPhysics = serde_json::from_str(&format!("\"{text}\"")).unwrap();
        assert_eq!(parsed, expect, "{text}");
        if !matches!(text, "smb1_nes" | "nesticles") {
            let encoded = serde_json::to_string(&expect).unwrap();
            assert_eq!(encoded, format!("\"{text}\""));
        }
    }
    let power: PowerUpStyle = serde_json::from_str("\"smb1\"").unwrap();
    assert_eq!(power, PowerUpStyle::Smb1);
    let brain: EnemyAiStyle = serde_json::from_str("\"smb1_nes\"").unwrap();
    assert_eq!(brain, EnemyAiStyle::Smb1Nes);

    let raw = r#"{
        "name": "Preset course",
        "character_physics": "smb1_peach",
        "power_up_style": "smb1",
        "enemy_ai": "smb1_nes",
        "levels": [{
            "id": "one",
            "name": "One",
            "w": 16,
            "h": 15,
            "player": {"x": 2.0, "y": 12.0}
        }]
    }"#;
    let loaded: Project = serde_json::from_str(raw).unwrap();
    assert_eq!(loaded.character_physics, CharacterPhysics::Smb1Peach);
    assert_eq!(loaded.power_up_style, PowerUpStyle::Smb1);
    assert_eq!(loaded.enemy_ai, EnemyAiStyle::Smb1Nes);
    assert_eq!(loaded.levels[0].enemy_ai, EnemyAiStyle::Smb1Nes);
    let pretty = serde_json::to_string_pretty(&loaded).unwrap();
    let again: Project = serde_json::from_str(&pretty).unwrap();
    assert_eq!(again.character_physics, CharacterPhysics::Smb1Peach);
    assert_eq!(again.power_up_style, PowerUpStyle::Smb1);
    assert_eq!(again.enemy_ai, EnemyAiStyle::Smb1Nes);
    assert_eq!(again.levels[0].id, "one");
}
