use super::*;
use crate::blueprint::{assign_cell, compose_fixture, LegacyBlock};
use crate::demo::demo_project;
use crate::engine::res_1080p::rules::smb1::collision::BlockKind;
use crate::engine::res_1080p::rules::smb1::hazards::HazardKind;
use crate::engine::res_1080p::{CharacterId, World};
use crate::engine::Pad;
use crate::physics::{CharacterPhysics, Input, Sim};
use crate::project::{
    Asset, AssetKind, BlockTag, Cell, HazardTag, ItemTag, Level, PowerUpStyle, Project, Spawn,
};

fn asset(id: &str) -> Asset {
    Asset {
        id: id.into(),
        name: "Picture".into(),
        kind: AssetKind::Terrain,
        style_id: String::new(),
        w: 16,
        h: 16,
        rgba_base64: String::new(),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: false,
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
        ..Cell::default()
    }
}

fn course(id: &str, w: u32, h: u32, cells: Vec<Cell>) -> Level {
    Level {
        id: id.into(),
        name: id.into(),
        style_id: String::new(),
        w,
        h,
        sky: "#24343c".into(),
        music_id: None,
        player: Spawn { x: 2.0, y: 12.0 },
        cells,
        enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
        entrances: Vec::new(),
        player_blueprint_id: None,
    }
}

fn project(levels: Vec<Level>) -> Project {
    let mut project = Project::blank("entrances");
    project.assets = vec![asset("art")];
    project.levels = levels;
    project
}

fn pipe(project: &mut Project, level: &str, x: i32, y: i32, entry: EntryDirection) {
    let role = compose_fixture(Some(LegacyBlock::Solid), None, Some(pipe_hazard(entry))).unwrap();
    assign_cell(project, level, x, y, role).unwrap();
}

fn stand(world: &mut World, x: i32, y: i32) {
    world.body.x = x * 72 * 4096 + 9 * 4096;
    world.body.y = y * 72 * 4096;
    world.body.vx = 0;
    world.body.vy = 0;
    world.pipe_armed = true;
}

fn press(entry: EntryDirection) -> Pad {
    let mut pad = Pad::default();
    match entry {
        EntryDirection::Up => pad.up = true,
        EntryDirection::Down => pad.down = true,
        EntryDirection::Left => pad.left = true,
        EntryDirection::Right => pad.right = true,
    }
    pad
}

fn enter(entry: EntryDirection, pipe_at: (i32, i32), dest: (i32, i32), facing: Facing) {
    let (px, py) = pipe_at;
    let (ox, oy) = match entry {
        EntryDirection::Down => (px, py - 1),
        EntryDirection::Up => (px, py + 1),
        EntryDirection::Left => (px + 1, py),
        EntryDirection::Right => (px - 1, py),
    };
    let mut level = course("course", 24, 15, vec![cell("art", px, py)]);
    level.player = Spawn {
        x: ox as f32,
        y: oy as f32,
    };
    let mut project = project(vec![level]);
    pipe(&mut project, "course", px, py, entry);
    let entrance = add_entrance(&mut project, "course", "Far", dest.0, dest.1, facing).unwrap();
    set_exit(&mut project, "course", px, py, "", &entrance, entry).unwrap();
    let mut world = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert!(
        world
            .solids
            .iter()
            .any(|solid| solid.cell_x == px && solid.cell_y == py && !solid.broken),
        "the pipe stays solid"
    );
    assert_eq!(world.solids[0].block, BlockKind::Solid);
    stand(&mut world, ox, oy);
    let before = world.solids.len();
    world.tick(&press(entry));
    assert_eq!(world.solids.len(), before);
    assert!(world.solids.iter().all(|solid| !solid.broken));
    assert_eq!(world.body.x, dest.0 * 72 * 4096 + 9 * 4096);
    assert_eq!(world.body.y, dest.1 * 72 * 4096);
    assert_eq!(world.body.facing, facing.sign());
    assert_eq!(world.body.vx, 0);
    assert_eq!(world.body.vy, 0);
}

#[test]
fn multiple_entrances_keep_their_ids() {
    let mut project = project(vec![course("course", 24, 15, vec![])]);
    let main = add_entrance(&mut project, "course", "Main", 2, 12, Facing::Right).unwrap();
    let side = add_entrance(&mut project, "course", "Side", 18, 4, Facing::Left).unwrap();
    assert_ne!(main, side);
    let saved = serde_json::to_string(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.levels[0].entrances.len(), 2);
    assert_eq!(loaded.levels[0].entrances[0].id, main);
    assert_eq!(loaded.levels[0].entrances[1].facing, Facing::Left);
    assert!(add_entrance(&mut project, "course", "Out", 40, 1, Facing::Right).is_err());
}

#[test]
fn one_way_exit_does_not_create_a_return() {
    let mut project = project(vec![
        course("a", 20, 12, vec![cell("art", 3, 8)]),
        course("b", 20, 12, vec![]),
    ]);
    pipe(&mut project, "a", 3, 8, EntryDirection::Down);
    let door = add_entrance(&mut project, "b", "Door", 4, 6, Facing::Right).unwrap();
    set_exit(&mut project, "a", 3, 8, "b", &door, EntryDirection::Down).unwrap();
    assert_eq!(project.transitions.len(), 1);
    assert!(project.levels[1].cells.is_empty());
}

#[test]
fn paired_connections_travel_both_ways_and_a_same_course_exit_moves_inside_it() {
    let mut project = project(vec![
        course("a", 30, 16, vec![cell("art", 4, 10), cell("art", 16, 10)]),
        course("b", 24, 15, vec![cell("art", 6, 9)]),
    ]);
    pipe(&mut project, "a", 4, 10, EntryDirection::Down);
    pipe(&mut project, "a", 16, 10, EntryDirection::Down);
    pipe(&mut project, "b", 6, 9, EntryDirection::Down);
    let far = add_entrance(&mut project, "a", "Far", 16, 9, Facing::Left).unwrap();
    let dock = add_entrance(&mut project, "b", "Dock", 6, 8, Facing::Right).unwrap();
    let gone = set_exit(&mut project, "a", 4, 10, "b", &dock, EntryDirection::Down).unwrap();
    set_exit(&mut project, "a", 16, 10, "", &far, EntryDirection::Down).unwrap();
    let home = add_entrance(&mut project, "a", "Home", 4, 9, Facing::Right).unwrap();
    let back = add_return(&mut project, &gone, 6, 9, &home, EntryDirection::Down).unwrap();
    assert_ne!(gone, back);
    assert_eq!(project.transitions.len(), 3);

    let mut same = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    same.body.power = crate::engine::res_1080p::character_physics::Power::Super;
    stand(&mut same, 16, 9);
    same.tick(&press(EntryDirection::Down));
    assert!(!same.pending);
    assert_eq!(same.body.x, 16 * 72 * 4096 + 9 * 4096);
    assert_eq!(same.body.facing, Facing::Left.sign());
    assert_eq!(
        same.body.power,
        crate::engine::res_1080p::character_physics::Power::Super
    );
    assert!(same.camera_x >= 0);

    let mut away = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert!(away
        .solids
        .iter()
        .any(|solid| solid.cell_x == 4 && !solid.broken));
    stand(&mut away, 4, 9);
    away.tick(&press(EntryDirection::Down));
    assert!(away.pending);
    assert_eq!(away.pending_level, "b");
    assert_eq!((away.pending_x, away.pending_y), (6, 8));
    assert_ne!((away.pending_x, away.pending_y), (2, 12));

    let mut back_world = World::from_level(&project, &project.levels[1], CharacterId::Mario);
    assert!(back_world.solids.iter().any(|solid| !solid.broken));
    stand(&mut back_world, 6, 8);
    back_world.tick(&press(EntryDirection::Down));
    assert_eq!(back_world.pending_level, "a");
    assert_eq!((back_world.pending_x, back_world.pending_y), (4, 9));
}

#[test]
fn pipe_down_enters_while_the_pipe_stays_solid() {
    enter(EntryDirection::Down, (4, 10), (12, 4), Facing::Right);
}

#[test]
fn pipe_up_enters_while_the_pipe_stays_solid() {
    enter(EntryDirection::Up, (4, 8), (12, 4), Facing::Left);
}

#[test]
fn pipe_left_enters_while_the_pipe_stays_solid() {
    enter(EntryDirection::Left, (8, 10), (2, 6), Facing::Left);
}

#[test]
fn pipe_right_enters_while_the_pipe_stays_solid() {
    enter(EntryDirection::Right, (8, 10), (18, 6), Facing::Right);
}

#[test]
fn a_missing_exit_does_not_move_the_player() {
    let mut project = project(vec![course("course", 20, 12, vec![cell("art", 4, 8)])]);
    pipe(&mut project, "course", 4, 8, EntryDirection::Down);
    project.levels[0].cells[0].transition_id = Some("gone".into());
    let mut world = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert!(world
        .blueprint_faults
        .iter()
        .any(|fault| fault.contains("will not be moved")));
    assert!(!world.hazards[0].warp_set);
    stand(&mut world, 4, 7);
    let y = world.body.y;
    world.tick(&press(EntryDirection::Down));
    assert!(!world.pending);
    assert!(world.body.y.abs_diff(y) < 72 * 4096);
}

#[test]
fn a_referenced_entrance_cannot_be_removed_and_a_broken_one_does_not_warp() {
    let mut project = project(vec![course("course", 20, 12, vec![cell("art", 4, 8)])]);
    pipe(&mut project, "course", 4, 8, EntryDirection::Down);
    let door = add_entrance(&mut project, "course", "Door", 8, 4, Facing::Right).unwrap();
    set_exit(
        &mut project,
        "course",
        4,
        8,
        "",
        &door,
        EntryDirection::Down,
    )
    .unwrap();
    let refused = remove_entrance(&mut project, "course", &door).unwrap_err();
    assert!(refused.contains("Clear that exit"));
    project.levels[0].entrances.clear();
    let world = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert!(!world.hazards[0].warp_set);
    assert!(world
        .blueprint_faults
        .iter()
        .any(|fault| fault.contains("entrance")));
}

#[test]
fn legacy_coordinate_warps_migrate_once_and_keep_working() {
    let mut placed = cell("art", 4, 8);
    placed.warp_choice = PlacementWarp::Destination(crate::project::WarpTarget {
        level_id: String::new(),
        x: 10,
        y: 3,
    });
    let mut project = project(vec![course("course", 20, 12, vec![placed])]);
    pipe(&mut project, "course", 4, 8, EntryDirection::Down);
    project.levels[0].cells[0].warp_choice =
        PlacementWarp::Destination(crate::project::WarpTarget {
            level_id: String::new(),
            x: 10,
            y: 3,
        });
    let before = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert_eq!(
        (before.hazards[0].warp_x, before.hazards[0].warp_y),
        (10, 3)
    );
    ensure_legacy_transitions(&mut project);
    let ids: Vec<_> = project
        .transitions
        .iter()
        .map(|link| link.id.clone())
        .collect();
    let entrance_ids: Vec<_> = project.levels[0]
        .entrances
        .iter()
        .map(|entrance| entrance.id.clone())
        .collect();
    ensure_legacy_transitions(&mut project);
    assert_eq!(
        project
            .transitions
            .iter()
            .map(|link| link.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        project.levels[0]
            .entrances
            .iter()
            .map(|entrance| entrance.id.clone())
            .collect::<Vec<_>>(),
        entrance_ids
    );
    assert_eq!(project.transitions.len(), 1);
    let after = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert_eq!((after.hazards[0].warp_x, after.hazards[0].warp_y), (10, 3));
    assert!(after.hazards[0].warp_set);
    let saved = serde_json::to_string(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    ensure_legacy_transitions(&mut project);
    assert_eq!(loaded.transitions[0].id, project.transitions[0].id);
}

#[test]
fn arriving_on_a_return_opening_does_not_bounce_straight_back() {
    let mut project = project(vec![course(
        "course",
        30,
        16,
        vec![cell("art", 4, 10), cell("art", 18, 10)],
    )]);
    pipe(&mut project, "course", 4, 10, EntryDirection::Down);
    pipe(&mut project, "course", 18, 10, EntryDirection::Down);
    let far = add_entrance(&mut project, "course", "Far", 18, 9, Facing::Right).unwrap();
    let home = add_entrance(&mut project, "course", "Home", 4, 9, Facing::Left).unwrap();
    let gone = set_exit(
        &mut project,
        "course",
        4,
        10,
        "",
        &far,
        EntryDirection::Down,
    )
    .unwrap();
    add_return(&mut project, &gone, 18, 10, &home, EntryDirection::Down).unwrap();
    let mut world = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    stand(&mut world, 4, 9);
    world.tick(&press(EntryDirection::Down));
    assert_eq!(world.body.y, 9 * 72 * 4096);
    assert!(!world.pipe_armed);
    world.tick(&press(EntryDirection::Down));
    assert_eq!(world.body.x, 18 * 72 * 4096 + 9 * 4096);
    stand(&mut world, 1, 12);
    world.pipe_armed = false;
    world.tick(&Pad::default());
    assert!(world.pipe_armed);
    world.warp_lock = 0;
    stand(&mut world, 18, 9);
    world.tick(&press(EntryDirection::Down));
    assert_eq!(world.body.x, 4 * 72 * 4096 + 9 * 4096);
    assert_eq!(world.body.facing, Facing::Left.sign());
}

#[test]
fn entrances_and_exits_survive_save_and_load() {
    let mut project = project(vec![
        course("a", 20, 12, vec![cell("art", 3, 8)]),
        course("b", 20, 12, vec![cell("art", 5, 6)]),
    ]);
    pipe(&mut project, "a", 3, 8, EntryDirection::Right);
    pipe(&mut project, "b", 5, 6, EntryDirection::Left);
    let dock = add_entrance(&mut project, "b", "Dock", 4, 6, Facing::Left).unwrap();
    let home = add_entrance(&mut project, "a", "Home", 4, 8, Facing::Right).unwrap();
    let gone = set_exit(&mut project, "a", 3, 8, "b", &dock, EntryDirection::Right).unwrap();
    add_return(&mut project, &gone, 5, 6, &home, EntryDirection::Left).unwrap();
    let saved = serde_json::to_string(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.transitions.len(), 2);
    assert_eq!(loaded.levels[0].entrances[0].id, home);
    assert_eq!(loaded.levels[1].entrances[0].name, "Dock");
    match named_exit(&loaded, "a", &loaded.levels[0].cells[0]) {
        NamedExit::Ready(exit) => {
            assert_eq!(exit.level_id, "b");
            assert_eq!((exit.x, exit.y), (4, 6));
            assert_eq!(exit.facing, Facing::Left.sign());
        }
        other => panic!("expected a ready exit, got {other:?}"),
    }
}

#[test]
fn cross_course_exit_uses_the_play_tick_and_not_the_primary_spawn() {
    let mut level = course("a", 24, 15, vec![cell("art", 4, 10)]);
    level.player = Spawn { x: 4.0, y: 9.0 };
    let other = course("b", 24, 15, vec![]);
    let mut project = project(vec![level, other]);
    project.character_physics = CharacterPhysics::Smb1Mario;
    project.power_up_style = PowerUpStyle::Smb1;
    pipe(&mut project, "a", 4, 10, EntryDirection::Down);
    let dock = add_entrance(&mut project, "b", "Dock", 11, 7, Facing::Left).unwrap();
    set_exit(&mut project, "a", 4, 10, "b", &dock, EntryDirection::Down).unwrap();
    let mut sim = Sim::from_level(&project, &project.levels[0]);
    sim.tick_play(
        CharacterPhysics::Smb1Mario,
        PowerUpStyle::Smb1,
        &Input {
            down: true,
            ..Input::default()
        },
    );
    let arrival = sim.take_warp().expect("cross-course exit");
    assert_eq!(arrival.level_id, "b");
    assert_eq!((arrival.x, arrival.y), (11, 7));
    assert_eq!(arrival.facing, -1);
    assert_ne!((arrival.x, arrival.y), (2, 12));
}

#[test]
fn a_wide_course_retargets_the_camera_after_a_same_course_exit() {
    let mut level = course("course", 80, 20, vec![cell("art", 4, 10)]);
    level.player = Spawn { x: 4.0, y: 9.0 };
    let mut project = project(vec![level]);
    pipe(&mut project, "course", 4, 10, EntryDirection::Down);
    let far = add_entrance(&mut project, "course", "Far", 60, 8, Facing::Left).unwrap();
    set_exit(
        &mut project,
        "course",
        4,
        10,
        "",
        &far,
        EntryDirection::Down,
    )
    .unwrap();
    let mut world = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    let start_camera = world.camera_x;
    stand(&mut world, 4, 9);
    world.tick(&press(EntryDirection::Down));
    assert!(world.camera_x > start_camera);
    assert_eq!(world.body.facing, -1);
    world.place_facing(2, 9, 1);
    assert_eq!(world.camera_x, 0);
    assert!(!world.pipe_armed);
}

#[test]
fn seed_spirit_still_starts_without_named_exits() {
    let project = demo_project();
    assert!(project.transitions.is_empty());
    assert!(project
        .levels
        .iter()
        .all(|level| level.entrances.is_empty()));
    let mut sim = Sim::from_level(&project, &project.levels[0]);
    for _ in 0..30 {
        sim.tick_play(
            CharacterPhysics::Smb1Mario,
            PowerUpStyle::Smb1,
            &Input::default(),
        );
    }
    assert!(sim.take_warp().is_none());
}

#[test]
fn hazard_kinds_cover_the_four_openings() {
    let _ = HazardKind::PipeDown;
    let _ = HazardKind::PipeUp;
    let _ = HazardKind::PipeLeft;
    let _ = HazardKind::PipeRight;
}
