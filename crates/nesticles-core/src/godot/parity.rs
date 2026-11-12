//! Traces from [`crate::physics::Sim::tick_play`] and a Godot project that replays them.
//!
//! The numbers are Nesticles' SMB1-style 1080p rules. They are the play-tab step,
//! not a claim that the constants were measured on NES hardware.

#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;

use serde::Serialize;

#[cfg(test)]
use super::generate::{prepare_output, write_inside};
use crate::engine::res_1080p::character_physics::{params_for, STOMP_VY, STOMP_VY_HELD};
#[cfg(test)]
use crate::engine::res_1080p::course_px;
use crate::engine::res_1080p::rules::smb1::enemies::{Kind, MAX_FALL, WALKER_GRAVITY, WALKER_VX};
use crate::engine::res_1080p::{
    World, CORNER_FP, FP, HIT_H, HIT_W, NUDGE_FP, SCREEN_H_FP, SCREEN_W_FP, TILE_FP, VIEW_MARGIN,
};
#[cfg(test)]
use crate::physics::{CharacterPhysics, Input, Sim};
#[cfg(test)]
use crate::project::PowerUpStyle;
#[cfg(test)]
use crate::project::{Asset, AssetKind, BlockTag, Cell, HazardTag, ItemTag, Layer, Level, Spawn};

#[cfg(test)]
const SIM_SCRIPT: &str = include_str!("nesticles_sim.gd");
#[cfg(test)]
const PARITY_SCRIPT: &str = include_str!("parity_driver.gd");

#[cfg(test)]
const PARITY_PROJECT: &str = r#"; Nesticles Godot parity. This file is fixed and contains no project text.
config_version=5

[application]

config/name="Nesticles Godot Parity"
run/main_scene="res://scenes/parity.tscn"
config/features=PackedStringArray("4.7", "GL Compatibility")

[display]

window/size/viewport_width=1280
window/size/viewport_height=720

[physics]

common/physics_ticks_per_second=60

[rendering]

textures/canvas_textures/default_texture_filter=0
renderer/rendering_method="gl_compatibility"
environment/defaults/default_clear_color=Color(0.16, 0.22, 0.26, 1)
"#;

#[cfg(test)]
const PARITY_SCENE: &str = r#"[gd_scene load_steps=2 format=3]

[ext_resource type="Script" path="res://scripts/parity_driver.gd" id="1_driver"]

[node name="Parity" type="Node"]
script = ExtResource("1_driver")
"#;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SpaceFile {
    pub fp: i32,
    pub tile_fp: i32,
    pub hit_w: i32,
    pub hit_h: i32,
    pub screen_w_fp: i32,
    pub screen_h_fp: i32,
    pub corner_fp: i32,
    pub nudge_fp: i32,
    pub view_margin: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MotionFile {
    pub max_walk: i32,
    pub max_run: i32,
    pub walk_accel: i32,
    pub run_accel: i32,
    pub release_decel: i32,
    pub skid_decel: i32,
    pub jump_slow: i32,
    pub jump_fast: i32,
    pub jump_fast_at: i32,
    pub gravity_hold: i32,
    pub gravity_normal: i32,
    pub max_fall: i32,
    pub hover: bool,
    pub stomp_vy: i32,
    pub stomp_vy_held: i32,
    pub walker_vx: i32,
    pub walker_gravity: i32,
    pub walker_max_fall: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct BodyFp {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub vx: i32,
    pub vy: i32,
    pub facing: i32,
    pub on_ground: bool,
    pub is_jumping: bool,
    pub run_timer: i32,
    pub hover_timer: i32,
    pub lives: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct EnemyFp {
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub vx: i32,
    pub vy: i32,
    pub facing: i32,
    pub alive: bool,
    pub awake: bool,
    pub on_ground: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SolidFp {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub cell_x: i32,
    pub cell_y: i32,
    pub broken: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct RectFp {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SimFile {
    pub space: SpaceFile,
    pub motion: MotionFile,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub level_w: i32,
    pub level_h: i32,
    pub won: bool,
    pub player: BodyFp,
    pub solids: Vec<SolidFp>,
    pub enemies: Vec<EnemyFp>,
    pub goals: Vec<RectFp>,
}

#[cfg(test)]
#[derive(Clone, Debug, Serialize)]
struct Sample {
    player: BodyFp,
    enemies: Vec<EnemyFp>,
    won: bool,
    jump: bool,
    stomp: bool,
    hurt: bool,
    goal: bool,
}

#[cfg(test)]
#[derive(Clone, Copy, Serialize)]
struct PadFile {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    jump_held: bool,
    jump_pressed: bool,
    run_held: bool,
}

#[cfg(test)]
#[derive(Serialize)]
struct CaseFile {
    name: String,
    state: SimFile,
    ticks: Vec<PadFile>,
    expect: Vec<Sample>,
}

#[cfg(test)]
#[derive(Serialize)]
struct ParityFile {
    format: u32,
    cases: Vec<CaseFile>,
}

pub(crate) fn sim_file_from_world(world: &World) -> SimFile {
    SimFile {
        space: space_file(),
        motion: motion_file(world.character),
        spawn_x: world.spawn_x,
        spawn_y: world.spawn_y,
        level_w: world.level_w,
        level_h: world.level_h,
        won: world.won,
        player: body_fp(&world.body),
        solids: world.solids.iter().map(solid_fp).collect(),
        enemies: world
            .enemies
            .iter()
            .filter(|enemy| enemy.kind == Kind::Goomba)
            .map(enemy_fp)
            .collect(),
        goals: world.goals.iter().map(rect_fp).collect(),
    }
}

pub(crate) fn skipped_enemy_kinds(world: &World) -> Vec<String> {
    let mut names = Vec::new();
    for enemy in &world.enemies {
        if enemy.kind == Kind::Goomba {
            continue;
        }
        let name = format!("{:?}", enemy.kind);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Write the headless parity project. It contains no pictures and no user strings.
#[cfg(test)]
pub(crate) fn generate_parity(dest: &Path) -> Result<(), String> {
    let document = ParityFile {
        format: 1,
        cases: cases(),
    };
    let json = serde_json::to_vec_pretty(&document)
        .map_err(|err| format!("Could not encode the parity traces: {err}"))?;
    prepare_output(dest)?;
    write_inside(dest, "project.godot", PARITY_PROJECT.as_bytes())?;
    write_inside(dest, "scripts/nesticles_sim.gd", SIM_SCRIPT.as_bytes())?;
    write_inside(dest, "scripts/parity_driver.gd", PARITY_SCRIPT.as_bytes())?;
    write_inside(dest, "scenes/parity.tscn", PARITY_SCENE.as_bytes())?;
    write_inside(dest, "data/parity.json", &json)?;
    write_inside(dest, ".nesticles-godot-out", b"phase6.1\n")?;
    Ok(())
}

#[cfg(test)]
fn cases() -> Vec<CaseFile> {
    let walk = hold(Input {
        right: true,
        ..Input::default()
    });
    let run = hold(Input {
        right: true,
        run_held: true,
        ..Input::default()
    });
    vec![
        record(
            "mario_walk",
            CharacterPhysics::Smb1Mario,
            ground(2.0, 12.0),
            &repeat_pad(&walk, 80),
        ),
        record(
            "mario_run",
            CharacterPhysics::Smb1Mario,
            ground(2.0, 12.0),
            &repeat_pad(&run, 45),
        ),
        record(
            "mario_face_left",
            CharacterPhysics::Smb1Mario,
            ground(2.0, 12.0),
            &[hold(Input {
                left: true,
                ..Input::default()
            })],
        ),
        record(
            "mario_jump_gravity",
            CharacterPhysics::Smb1Mario,
            ground(2.0, 12.0),
            &[
                Input {
                    jump_pressed: true,
                    jump_held: true,
                    ..Input::default()
                },
                Input {
                    jump_held: true,
                    ..Input::default()
                },
                Input::default(),
            ],
        ),
        record(
            "mario_fall",
            CharacterPhysics::Smb1Mario,
            ground(3.0, 4.0),
            &repeat_pad(&Input::default(), 34),
        ),
        record(
            "mario_skid",
            CharacterPhysics::Smb1Mario,
            ground(2.0, 12.0),
            &skid_pads(),
        ),
        record(
            "luigi_jump",
            CharacterPhysics::Smb1Luigi,
            ground(2.0, 12.0),
            &[Input {
                jump_pressed: true,
                jump_held: true,
                ..Input::default()
            }],
        ),
        record(
            "toad_walk",
            CharacterPhysics::Smb1Toad,
            ground(2.0, 12.0),
            &repeat_pad(&walk, 10),
        ),
        record(
            "peach_hover",
            CharacterPhysics::Smb1Peach,
            ground(2.0, 12.0),
            &hover_pads(46),
        ),
        record(
            "goomba_reverse",
            CharacterPhysics::Smb1Mario,
            goomba_wall(),
            &repeat_pad(&Input::default(), 41),
        ),
        record(
            "goomba_stomp",
            CharacterPhysics::Smb1Mario,
            goomba_stomp_course(),
            &repeat_pad(&Input::default(), 9),
        ),
        record(
            "goomba_bump",
            CharacterPhysics::Smb1Mario,
            goomba_bump_course(),
            &repeat_pad(&walk, 41),
        ),
        record(
            "goal_flag",
            CharacterPhysics::Smb1Mario,
            goal_course(),
            &repeat_pad(&walk, 55),
        ),
    ]
}

#[cfg(test)]
fn record(name: &str, preset: CharacterPhysics, level: Level, pads: &[Input]) -> CaseFile {
    let (_, mut sim) = stage(preset, level);
    let state = sim_file_from_world(sim.play_world().expect("play field"));
    let mut expect = Vec::with_capacity(pads.len());
    for pad in pads {
        sim.tick_play(preset, PowerUpStyle::Smb1, pad);
        expect.push(sample(sim.play_world().expect("play field")));
    }
    CaseFile {
        name: name.to_string(),
        state,
        ticks: pads.iter().copied().map(pad_file).collect(),
        expect,
    }
}

#[cfg(test)]
fn sample(world: &World) -> Sample {
    Sample {
        player: body_fp(&world.body),
        enemies: world
            .enemies
            .iter()
            .filter(|enemy| enemy.kind == Kind::Goomba)
            .map(enemy_fp)
            .collect(),
        won: world.won,
        jump: world.events.jump,
        stomp: world.events.stomp,
        hurt: world.events.hurt,
        goal: world.events.goal,
    }
}

fn space_file() -> SpaceFile {
    SpaceFile {
        fp: FP,
        tile_fp: TILE_FP,
        hit_w: HIT_W,
        hit_h: HIT_H,
        screen_w_fp: SCREEN_W_FP,
        screen_h_fp: SCREEN_H_FP,
        corner_fp: CORNER_FP,
        nudge_fp: NUDGE_FP,
        view_margin: VIEW_MARGIN,
    }
}

fn motion_file(id: crate::engine::res_1080p::CharacterId) -> MotionFile {
    let params = params_for(id);
    MotionFile {
        max_walk: params.max_walk,
        max_run: params.max_run,
        walk_accel: params.walk_accel,
        run_accel: params.run_accel,
        release_decel: params.release_decel,
        skid_decel: params.skid_decel,
        jump_slow: params.jump_slow,
        jump_fast: params.jump_fast,
        jump_fast_at: params.jump_fast_at,
        gravity_hold: params.gravity_hold,
        gravity_normal: params.gravity_normal,
        max_fall: params.max_fall,
        hover: params.hover,
        stomp_vy: STOMP_VY,
        stomp_vy_held: STOMP_VY_HELD,
        walker_vx: WALKER_VX,
        walker_gravity: WALKER_GRAVITY,
        walker_max_fall: MAX_FALL,
    }
}

fn body_fp(body: &crate::engine::res_1080p::character_physics::Body) -> BodyFp {
    BodyFp {
        x: body.x,
        y: body.y,
        w: body.w,
        h: body.h,
        vx: body.vx,
        vy: body.vy,
        facing: body.facing,
        on_ground: body.on_ground,
        is_jumping: body.is_jumping,
        run_timer: body.run_timer,
        hover_timer: body.hover_timer,
        lives: body.lives,
    }
}

fn enemy_fp(enemy: &crate::engine::res_1080p::rules::smb1::enemies::EnemyActor) -> EnemyFp {
    EnemyFp {
        kind: "goomba".to_string(),
        x: enemy.x,
        y: enemy.y,
        w: enemy.w,
        h: enemy.h,
        vx: enemy.vx,
        vy: enemy.vy,
        facing: enemy.facing,
        alive: enemy.alive,
        awake: enemy.awake,
        on_ground: enemy.on_ground,
    }
}

fn solid_fp(solid: &crate::engine::res_1080p::rules::smb1::collision::Solid) -> SolidFp {
    SolidFp {
        x: solid.x,
        y: solid.y,
        w: TILE_FP,
        h: TILE_FP,
        cell_x: solid.cell_x,
        cell_y: solid.cell_y,
        broken: solid.broken,
    }
}

fn rect_fp(rect: &crate::engine::res_1080p::Rect) -> RectFp {
    RectFp {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
    }
}

#[cfg(test)]
fn pad_file(input: Input) -> PadFile {
    PadFile {
        left: input.left,
        right: input.right,
        up: input.up,
        down: input.down,
        jump_held: input.jump_held,
        jump_pressed: input.jump_pressed,
        run_held: input.run_held,
    }
}

#[cfg(test)]
fn hold(input: Input) -> Input {
    input
}

#[cfg(test)]
fn repeat_pad(input: &Input, count: usize) -> Vec<Input> {
    vec![*input; count]
}

#[cfg(test)]
fn skid_pads() -> Vec<Input> {
    let mut pads = repeat_pad(
        &Input {
            right: true,
            ..Input::default()
        },
        20,
    );
    pads.extend(repeat_pad(
        &Input {
            left: true,
            ..Input::default()
        },
        8,
    ));
    pads
}

#[cfg(test)]
fn hover_pads(count: usize) -> Vec<Input> {
    (0..count)
        .map(|index| Input {
            jump_pressed: index == 0,
            jump_held: true,
            ..Input::default()
        })
        .collect()
}

#[cfg(test)]
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

#[cfg(test)]
fn cell(id: &str, x: i32, y: i32) -> Cell {
    Cell {
        x,
        y,
        asset_id: id.into(),
        layer: Layer::Main,
        ..Cell::default()
    }
}

#[cfg(test)]
fn ground_row() -> Vec<Cell> {
    (0..24).map(|x| cell("ground", x, 13)).collect()
}

#[cfg(test)]
fn course(cells: Vec<Cell>, px: f32, py: f32) -> Level {
    Level {
        id: "course".into(),
        name: "Parity course".into(),
        style_id: String::new(),
        w: 24,
        h: 15,
        sky: "#24343c".into(),
        music_id: None,
        player: Spawn { x: px, y: py },
        cells,
        enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
        entrances: Vec::new(),
    }
}

#[cfg(test)]
fn ground(px: f32, py: f32) -> Level {
    course(ground_row(), px, py)
}

#[cfg(test)]
fn goomba_wall() -> Level {
    let mut cells = ground_row();
    cells.push(cell("goomba", 8, 12));
    cells.push(cell("ground", 6, 12));
    course(cells, 2.0, 12.0)
}

#[cfg(test)]
fn goomba_stomp_course() -> Level {
    let mut cells = ground_row();
    cells.push(cell("goomba", 5, 12));
    cells.push(cell("ground", 4, 12));
    cells.push(cell("ground", 6, 12));
    course(cells, 5.0, 10.0)
}

#[cfg(test)]
fn goomba_bump_course() -> Level {
    let mut cells = ground_row();
    cells.push(cell("goomba", 6, 12));
    course(cells, 2.0, 12.0)
}

#[cfg(test)]
fn goal_course() -> Level {
    let mut cells = ground_row();
    cells.push(cell("goal", 6, 12));
    course(cells, 2.0, 12.0)
}

#[cfg(test)]
fn stage(preset: CharacterPhysics, level: Level) -> (crate::project::Project, Sim) {
    let mut project = crate::project::Project::blank("parity");
    project.character_physics = preset;
    project.power_up_style = PowerUpStyle::Smb1;
    project.enemy_ai = crate::enemy_ai::EnemyAiStyle::Smb1Nes;
    project.assets = vec![
        asset("ground", "Ground", AssetKind::Terrain),
        asset("goomba", "Goomba", AssetKind::Enemy),
        asset("goal", "Flag", AssetKind::Goal),
    ];
    project.levels = vec![level];
    let sim = Sim::from_level(&project, &project.levels[0]);
    (project, sim)
}

#[cfg(test)]
fn near(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 0.001
}

#[cfg(test)]
fn speed(units: i32) -> f32 {
    course_px(units) * 60.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(name: &str) -> CaseFile {
        cases()
            .into_iter()
            .find(|case| case.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    #[test]
    fn traces_match_the_play_tab_baselines() {
        let walk = one("mario_walk");
        let last = walk.expect.last().unwrap();
        assert!(
            near(speed(last.player.vx), 90.0),
            "{}",
            speed(last.player.vx)
        );
        assert!(near(course_px(last.player.x), 124.4296875));
        assert!(near(course_px(last.player.y), 192.0));
        assert!(last.player.on_ground);
        assert_eq!(last.player.facing, 1);

        let run = one("mario_run");
        let last = run.expect.last().unwrap();
        assert!(near(speed(last.player.vx), 150.0));
        assert!(near(course_px(last.player.x), 91.607421875));

        let jump = one("mario_jump_gravity");
        assert!(jump.expect[0].jump);
        assert!(near(speed(jump.expect[0].player.vy), -232.5));
        assert!(near(speed(jump.expect[1].player.vy), -225.0));
        assert!(near(speed(jump.expect[2].player.vy), -198.75));
        assert!(!jump.expect[2].player.on_ground);

        let fall = one("mario_fall");
        assert!(!fall.expect[32].player.on_ground);
        assert!(fall.expect[33].player.on_ground);
        assert!(near(course_px(fall.expect[33].player.y), 192.0));
        assert!(near(speed(fall.expect[33].player.vy), 0.0));

        let luigi = one("luigi_jump");
        assert!(near(speed(luigi.expect[0].player.vy), -260.6673177));
        let toad = one("toad_walk");
        assert!(near(speed(toad.expect[9].player.vx), 31.1848958));
        let peach = one("peach_hover");
        let hover = speed(peach.expect[45].player.vy).abs();
        assert!(near(hover, 10.0 / 3.0), "{hover}");
        assert!(!peach.expect[45].player.on_ground);

        let goomba = one("goomba_reverse");
        assert_eq!(goomba.expect[0].enemies[0].facing, -1);
        assert!(near(course_px(goomba.expect[0].enemies[0].x), 129.5));
        assert!(near(course_px(goomba.expect[35].enemies[0].x), 112.0));
        assert_eq!(goomba.expect[40].enemies[0].facing, 1);
        assert!(near(course_px(goomba.expect[40].enemies[0].x), 114.0));
        assert!(goomba.expect[40].enemies[0].alive);

        let stomp = one("goomba_stomp");
        assert!(stomp.expect[8].stomp);
        assert!(!stomp.expect[8].enemies[0].alive);
        assert!(near(speed(stomp.expect[8].player.vy), -240.0));
        assert!(!stomp.expect[8].hurt);

        let bump = one("goomba_bump");
        assert!(bump.expect[40].hurt);
        assert!(bump.expect[40].enemies[0].alive);
        assert_eq!(bump.expect[40].player.x, bump.state.player.x);
        assert_eq!(bump.expect[40].player.lives, 2);

        let goal = one("goal_flag");
        assert!(goal.expect[53].goal);
        assert!(goal.expect[53].won);
        assert!(near(course_px(goal.expect[53].player.x), 85.4296875));
        assert_eq!(goal.expect[53].player.vx, 0);
        assert!(goal.expect[54].won);
        assert!(!goal.expect[54].goal);
        assert_eq!(goal.expect[54].player.x, goal.expect[53].player.x);
    }

    #[test]
    fn godot_replays_the_tick_play_traces() {
        let tool = match crate::godot::discover(None) {
            Ok(tool) => tool,
            Err(err) => panic!(
                "Godot {} is required for the parity replay: {err}",
                crate::godot::PINNED_VERSION
            ),
        };
        let dest =
            std::env::temp_dir().join(format!("nesticles-godot-parity-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);
        generate_parity(&dest).unwrap();
        let output = crate::godot::run_parity(&tool.executable, &dest)
            .unwrap_or_else(|err| panic!("parity replay failed to start: {err}"));
        assert!(
            output
                .stdout
                .contains("NESTICLES_PARITY_OK cases=13 ticks=394"),
            "stdout:\n{}\nstderr:\n{}",
            output.stdout,
            output.stderr
        );
        assert!(
            !output.stdout.contains("NESTICLES_PARITY_FAIL"),
            "{}\n{}",
            output.stdout,
            output.stderr
        );
        assert_eq!(
            output.status,
            Some(0),
            "{}\n{}",
            output.stdout,
            output.stderr
        );
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        assert!(!combined.contains("SCRIPT ERROR"), "{combined}");
        let _ = fs::remove_dir_all(&dest);
    }
}
